//! UEFI Tables

use core::{marker::PhantomData, mem::size_of, ptr::null_mut, time::Duration};

use crate::{
    error::{EfiStatus, Result, UefiError},
    proto::{self, console::SimpleTextOutput, Guid, Protocol, Scope},
    string::UefiStr,
    util::interface,
    EfiHandle,
};

pub mod raw;
use alloc::{string::String, vec::Vec};

use raw::*;

interface!(
    /// The UEFI Boot Services
    BootServices(RawBootServices),
);

// Internal
impl<'table> BootServices<'table> {
    /// Raw `locate_handle` wrapper
    ///
    /// # Safety
    ///
    /// Arguments must be correct for [`LocateSearch`]
    unsafe fn locate_handle(
        &self,
        search: LocateSearch,
        search_key: *mut u8,
        guid: *const Guid,
    ) -> Result<Vec<EfiHandle>> {
        let lh = self.interface().locate_handle.unwrap();
        let key = search_key;
        // Note: This is in bytes.
        let mut size = 0;
        let mut out: Vec<EfiHandle> = Vec::new();
        let guid_ptr = guid;

        // Get buffer size
        let ret = unsafe { (lh)(search, guid_ptr, key, &mut size, null_mut()) };
        if ret == EfiStatus::NOT_FOUND {
            // No handles matched the search
            return Ok(out);
        } else if ret != EfiStatus::BUFFER_TOO_SMALL {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }

        // Reserve enough elements
        let elems = size / size_of::<EfiHandle>();
        out.resize(elems, EfiHandle(null_mut()));

        // Fill our array
        let ret = unsafe { (lh)(search, guid_ptr, key, &mut size, out.as_mut_ptr()) };
        if ret.is_success() {
            Ok(out)
        } else if ret == EfiStatus::NOT_FOUND {
            Ok(Vec::new())
        } else {
            Err(ret.into())
        }
    }
}

/// Protocol handling
impl<'table> BootServices<'table> {
    /// Get every handle on the system
    pub fn all_handles(&self) -> Result<Vec<EfiHandle>> {
        // Safety: Statically correct for this call
        // All parameters are ignored for ALL_HANDLES
        unsafe { self.locate_handle(LocateSearch::ALL_HANDLES, null_mut(), null_mut()) }
    }

    /// Get every handle that support the [`Protocol`]
    pub fn handles_for_protocol<Proto: Protocol<'table>>(&self) -> Result<Vec<EfiHandle>> {
        let guid = Proto::GUID;
        // Safety: Statically correct for this call
        // `search_key` is ignored for BY_PROTOCOL
        unsafe { self.locate_handle(LocateSearch::BY_PROTOCOL, null_mut(), &guid) }
    }

    /// Find and return an arbitrary protocol instance from an arbitrary handle
    /// matching `guid`.
    ///
    /// This is useful for protocols that don't care about where they're
    /// attached, or where only one handle is expected to exist.
    ///
    /// This is shorthand for
    ///
    /// TODO: Section about finding handles for protocols
    ///
    /// If no protocol is found, [None] is returned.
    pub fn locate_protocol<'boot, Protocol: proto::Protocol<'boot>>(
        &'boot self,
    ) -> Result<Option<Protocol>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = Protocol::GUID;
        let lp = self.interface().locate_protocol.unwrap();
        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe { (lp)(&mut guid, null_mut(), &mut out) };
        if ret.is_success() {
            assert!(
                !out.is_null(),
                "UEFI locate_protocol returned success, but the protocol was null. \
                The Protocol was \"{}\" with GUID `{}`",
                Protocol::NAME,
                Protocol::GUID.to_uuid()
            );
            // Safety:
            // - Success means `out` is valid
            // - We assert its not null just in case.
            unsafe { Ok(Some(Protocol::from_raw(out as *mut Protocol::Raw))) }
        } else if ret == EfiStatus::NOT_FOUND {
            Ok(None)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Open the protocol on `handle`, if it exists, on behalf of `agent`.
    ///
    /// For applications, `agent` is your image handle.
    /// `controller` is [`None`].
    ///
    /// For drivers, `agent` is the handle with `EFI_DRIVER_BINDING_PROTOCOL`.
    /// `controller` is the controller handle that requires `Proto`
    ///
    /// The protocol is opened in Exclusive mode
    // FIXME: This method is actually incompatible with drivers. Have two separate
    // ones
    // TODO: Is this safe/sound to call with the same protocol twice?
    // Do we need to test the protocol first?
    // *Seems* to be fine, in qemu?
    pub fn open_protocol<'boot, Proto: proto::Protocol<'boot>>(
        &'boot self,
        handle: EfiHandle,
        agent: EfiHandle,
        controller: Option<EfiHandle>,
    ) -> Result<Option<Scope<Proto>>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = Proto::GUID;
        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe {
            (self.interface().open_protocol.unwrap())(
                handle,
                &mut guid,
                &mut out,
                agent,
                controller.unwrap_or(EfiHandle(null_mut())),
                0x20,
            )
        };
        if ret.is_success() {
            // Safety: Success means out is valid
            unsafe {
                Ok(Some(Scope::new(
                    Proto::from_raw(out as *mut Proto::Raw),
                    handle,
                    agent,
                    controller,
                )))
            }
        } else if ret == EfiStatus::UNSUPPORTED {
            Ok(None)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Close the [crate::proto::Protocol] on `handle`
    ///
    /// `handle`, `agent`, and `controller` must be the same [EfiHandle]'s
    /// passed to [`BootServices::open_protocol`]
    pub fn close_protocol<'boot, T: proto::Protocol<'boot>>(
        &self,
        handle: EfiHandle,
        agent: EfiHandle,
        controller: Option<EfiHandle>,
    ) -> Result<()> {
        let mut guid = T::GUID;
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe {
            (self.interface().close_protocol.unwrap())(
                handle,
                &mut guid,
                agent,
                controller.unwrap_or(EfiHandle(null_mut())),
            )
        }
        .into()
    }

    /// Install an instance of [proto::Protocol] on `handle`
    pub fn install_protocol<'a, T: proto::Protocol<'a>>(
        &self,
        handle: EfiHandle,
        interface: &'static mut T::Raw,
    ) -> Result<()> {
        // Safety:
        // `interface` being a static mut reference guarantees validity and lifetime.
        unsafe { self.install_protocol_ptr::<T>(handle, interface) }
    }

    /// Install a `Protocol` on `handle`
    ///
    /// # Safety
    ///
    /// - Pointer must be a valid instance of [proto::Protocol]
    /// - Pointer must live long enough
    pub unsafe fn install_protocol_ptr<'a, T: proto::Protocol<'a>>(
        &self,
        handle: EfiHandle,
        interface: *mut T::Raw,
    ) -> Result<()> {
        let mut guid = T::GUID;
        let mut h = handle;

        (self.interface().install_protocol_interface.unwrap())(
            &mut h,
            &mut guid,
            0,
            interface as *mut u8,
        )
        .into()
    }

    /// Query `handle` to determine if it supports `Protocol`
    ///
    /// If no protocol is found, [`Ok(None)`] is returned.
    // TODO: Is this safe/sound to call with the same protocol twice?
    // Same reasons as `open_protocol`
    pub fn handle_protocol<'boot, Protocol: proto::Protocol<'boot>>(
        &'boot self,
        handle: EfiHandle,
    ) -> Result<Option<Protocol>> {
        let mut out: *mut u8 = null_mut();
        let guid = Protocol::GUID;
        let lp = self.interface().handle_protocol.unwrap();
        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe { (lp)(handle, &guid, &mut out) };
        if ret.is_success() {
            assert!(
                !out.is_null(),
                "UEFI handle_protocol returned success, but the protocol was null. \
                The Protocol was \"{}\" with GUID `{}`",
                Protocol::NAME,
                Protocol::GUID.to_uuid()
            );
            // Safety:
            // - Success means `out` is valid
            // - We assert its not null just in case.
            unsafe { Ok(Some(Protocol::from_raw(out as *mut Protocol::Raw))) }
        } else if ret == EfiStatus::UNSUPPORTED {
            Ok(None)
        } else {
            Err(UefiError::new(ret))
        }
    }
}

impl<'table> BootServices<'table> {
    /// Exit the image represented by `handle` with `status`
    pub fn exit(&self, handle: EfiHandle, status: EfiStatus) -> Result<()> {
        // Safety: Construction ensures safety
        unsafe { (self.interface().exit.unwrap())(handle, status, 0, null_mut()) }.into()
    }

    /// Stall for [`Duration`]
    ///
    /// Returns [`EfiStatus::INVALID_PARAMETER`] if `dur` does not fit in
    /// [usize]
    pub fn stall(&self, dur: Duration) -> Result<()> {
        let time = match dur
            .as_micros()
            .try_into()
            .map_err(|_| EfiStatus::INVALID_PARAMETER)
        {
            Ok(t) => t,
            Err(e) => return e.into(),
        };
        // Safety: Construction ensures safety
        unsafe { (self.interface().stall.unwrap())(time) }.into()
    }

    /// The next monotonic count
    pub fn next_monotonic_count(&self) -> Result<u64> {
        let mut out = 0;
        // Safety: Construction ensures safety
        let ret = unsafe { (self.interface().get_next_monotonic_count.unwrap())(&mut out) };
        if ret.is_success() {
            return Ok(out);
        }
        Err(UefiError::new(ret))
    }

    /// Set the watchdog timer. [`None`] disables the timer.
    pub fn set_watchdog(&self, timeout: Option<Duration>) -> Result<()> {
        let timeout = timeout.unwrap_or_default();
        let secs = match timeout
            .as_secs()
            .try_into()
            .map_err(|_| EfiStatus::INVALID_PARAMETER)
        {
            Ok(t) => t,
            Err(e) => return e.into(),
        };
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe { (self.interface().set_watchdog_timer.unwrap())(secs, 0x10000, 0, null_mut()) }
            .into()
    }

    /// Allocate `size` bytes of memory from pool of type `ty`
    pub fn allocate_pool(&self, ty: crate::mem::MemoryType, size: usize) -> Result<*mut u8> {
        let mut out: *mut u8 = null_mut();
        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe { (self.interface().allocate_pool.unwrap())(ty, size, &mut out) };
        if ret.is_success() {
            Ok(out)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Free memory allocated by [BootServices::allocate_pool]
    ///
    /// # Safety
    ///
    /// - Must have been allocated by [BootServices::allocate_pool]
    /// - Must be non-null
    pub unsafe fn free_pool(&self, memory: *mut u8) -> Result<()> {
        (self.interface().free_pool.unwrap())(memory).into()
    }

    /// Load an image from memory `src`, returning its handle.
    ///
    /// Note that this will return [Ok] on a [`EfiStatus::SECURITY_VIOLATION`].
    ///
    /// You will need to handle that case in [`BootServices::start_image`]
    pub fn load_image(&self, parent: EfiHandle, src: &[u8]) -> Result<EfiHandle> {
        let mut out = EfiHandle(null_mut());
        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe {
            (self.interface().load_image.unwrap())(
                false,
                parent,
                // TODO: Provide fake device path
                // This makes nicer debugging?, and images may expect it
                null_mut(),
                // UEFI pls do not modify us.
                src.as_ptr() as *mut _,
                src.len(),
                &mut out,
            )
        };

        if ret.is_success() || ret == EfiStatus::SECURITY_VIOLATION {
            assert_ne!(out, EfiHandle(null_mut()));
            Ok(out)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Unload an earlier loaded image
    pub fn start_image(&self, handle: EfiHandle) -> Result<()> {
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe { (self.interface().start_image.unwrap())(handle, &mut 0, null_mut()).into() }
    }

    /// Unload an earlier loaded image
    pub fn unload_image(&self, handle: EfiHandle) -> Result<()> {
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe { (self.interface().unload_image.unwrap())(handle).into() }
    }
}

interface!(
    // /// The UEFI Runtime Services
    // RuntimeServices(RawRuntimeServices),
);

/// Type marker for [`SystemTable`] representing before ExitBootServices is
/// called
pub struct Boot;

/// Type marker for [`SystemTable`] representing after ExitBootServices is
/// called
pub struct Runtime;

/// Internal state for global handling code
pub(crate) struct Internal;

/// The UEFI System table
///
/// This is your entry-point to using UEFI and all its services
// NOTE: This CANNOT be Copy or Clone, as this would violate the planned
// safety guarantees of passing it to ExitBootServices
#[derive(Debug)]
#[repr(transparent)]
pub struct SystemTable<State> {
    /// Pointer to the table.
    ///
    /// Conceptually, this is static, it will be alive for the life of the
    /// program.
    ///
    /// That said, it would be problematic if this was a static reference,
    /// because it can change out from under us, such as when ExitBootServices
    /// is called.
    table: *mut RawSystemTable,

    phantom: PhantomData<*const State>,
}

impl<T> SystemTable<T> {
    /// Create new SystemTable
    ///
    /// # Safety
    ///
    /// - Must be valid non-null pointer
    pub(crate) unsafe fn new(this: *mut RawSystemTable) -> Self {
        Self {
            table: this,
            phantom: PhantomData,
        }
    }

    fn table(&self) -> &RawSystemTable {
        // Safety:
        // - Never null
        // - Pointer will always be valid in the `Boot` state
        // In the `Runtime` state it becomes the users responsibility?
        // Or out of scope since it depends on CPU execution environment?
        // Specifics figured out later
        unsafe { &*self.table }
    }
}

impl SystemTable<Internal> {
    /// Get the SystemTable if still in boot mode.
    ///
    /// This is useful for the logging, panic, and alloc error handlers
    ///
    /// If ExitBootServices has NOT been called,
    /// return [`SystemTable<Boot>`], otherwise [`None`]
    pub(crate) fn as_boot(&self) -> Option<SystemTable<Boot>> {
        if !self.table().boot_services.is_null() {
            // Safety:
            // - Above check verifies ExitBootServices has not been called.
            Some(unsafe { SystemTable::new(self.table) })
        } else {
            None
        }
    }

    /// Get the SystemTable if not in boot mode.
    ///
    /// This is useful for the logging, panic, and alloc error handlers
    ///
    /// If ExitBootServices has NOT been called,
    /// return [`SystemTable<Runtime>`], otherwise [`None`]
    pub(crate) fn _as_runtime(&self) -> Option<SystemTable<Boot>> {
        if !self.table().boot_services.is_null() {
            // Safety:
            // - Above check verifies ExitBootServices has not been called.
            Some(unsafe { SystemTable::new(self.table) })
        } else {
            None
        }
    }
}

impl SystemTable<Boot> {
    /// String identifying the vendor
    pub fn firmware_vendor(&self) -> String {
        let p = self.table().firmware_vendor as *mut u16;
        if p.is_null() {
            return String::new();
        }
        // Safety: always valid
        unsafe { UefiStr::from_ptr(p) }.to_string()
    }

    /// Firmware-specific value indicating its revision
    pub fn firmware_revision(&self) -> u32 {
        self.table().firmware_revision
    }

    /// Returns the (Major, Minor) UEFI Revision that this implementation claims
    /// conformance to.
    pub fn uefi_revision(&self) -> (u32, u32) {
        (
            self.table().header.revision.major(),
            self.table().header.revision.minor(),
        )
    }

    /// Output on stdout
    pub fn stdout(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_out;
        assert!(!ptr.is_null(), "con_out handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr) }
    }

    /// Output on stderr
    pub fn stderr(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_err;
        assert!(!ptr.is_null(), "std_err handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr) }
    }

    /// Reference to the UEFI Boot services.
    pub fn boot(&self) -> BootServices<'_> {
        let ptr = self.table().boot_services;
        assert!(!ptr.is_null(), "boot_services handle was null");
        // Safety: Construction ensures safety.
        unsafe { BootServices::new(ptr) }
    }
}
