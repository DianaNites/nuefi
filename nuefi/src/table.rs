//! UEFI Tables

use core::{
    iter::from_fn,
    marker::PhantomData,
    mem::{size_of, transmute},
    ptr::{null_mut, NonNull},
    slice::from_raw_parts,
    time::Duration,
};

use crate::{
    error::{EfiStatus, Result, UefiError},
    get_image_handle,
    mem::MemoryType,
    proto::{
        self,
        console::SimpleTextOutput,
        device_path::{raw::RawDevicePath, DevicePath},
        Guid,
        Protocol,
        Scope,
    },
    string::UefiStr,
    util::interface,
    EfiHandle,
};

pub mod config;
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
        let lh = self
            .interface()
            .locate_handle
            .ok_or(EfiStatus::UNSUPPORTED)?;
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

    /// Load an image from memory `src`, returning its handle.
    ///
    /// Note that this will return [Ok] on a [`EfiStatus::SECURITY_VIOLATION`].
    ///
    /// This case will need to be handled in [`BootServices::start_image`]
    ///
    /// # Safety
    ///
    /// Arguments must be correct for [load_image][`BootServices::load_image`]
    unsafe fn load_image_impl(
        &self,
        policy: bool,
        devpath: *mut RawDevicePath,
        parent: EfiHandle,
        src: *mut u8,
        src_len: usize,
    ) -> Result<EfiHandle> {
        let mut out = EfiHandle(null_mut());
        let li = self.interface().load_image.ok_or(EfiStatus::UNSUPPORTED)?;

        // Safety: Callers responsibility
        let ret = (li)(policy, parent, devpath, src, src_len, &mut out);

        if ret.is_success() || ret == EfiStatus::SECURITY_VIOLATION {
            assert_ne!(out, EfiHandle(null_mut()));
            Ok(out)
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
    // TODO: / FIXME This method is actually incompatible with drivers. Have two separate
    // ones
    // TODO: Is this safe/sound to call with the same protocol twice?
    // Do we need to test the protocol first?
    // *Seems* to be fine, in qemu?
    #[cfg(no)]
    fn open_protocol() {}
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
    ///
    /// [`EfiStatus::NOT_FOUND`] is treated as success with a empty `Vec`
    pub fn handles_for_protocol<'boot, Proto: Protocol<'boot>>(&self) -> Result<Vec<EfiHandle>> {
        let guid = Proto::GUID;
        // Safety: Statically correct for this call
        // `search_key` is ignored for BY_PROTOCOL
        unsafe { self.locate_handle(LocateSearch::BY_PROTOCOL, null_mut(), &guid) }
    }

    /// Get an arbitrary handle that supports [`Protocol`]
    pub fn handle_for<'boot, Proto: Protocol<'boot>>(&self) -> Result<EfiHandle> {
        self.handles_for_protocol::<Proto>()?
            .first()
            .copied()
            .ok_or(EfiStatus::NOT_FOUND.into())
    }

    /// Find and return the first protocol instance found
    ///
    /// This is a safe replacement for [`BootServices::locate_protocol`].
    ///
    /// This will exclusively open the protocol.
    /// See [`BootServices::open_protocol`] for caveats.
    pub fn get_protocol<'boot, Protocol: proto::Protocol<'boot>>(
        &'boot self,
    ) -> Result<Scope<'boot, Protocol>> {
        self.open_protocol::<Protocol>(self.handle_for::<Protocol>()?)?
            .ok_or(EfiStatus::NOT_FOUND.into())
    }

    /// Find and return the first protocol instance found
    ///
    /// This finds the first handle that supports the requested protocol,
    /// and then unsafely returns an instance to it.
    ///
    /// See [`BootServices::get_protocol`] for the safe version of this.
    ///
    /// This is useful for protocols that don't care about where they're
    /// attached, or where only one handle is expected to exist.
    ///
    /// If no protocol is found, [None] is returned.
    ///
    /// # Safety
    ///
    /// This is unsafe because, like [`BootServices::handle_protocol`],
    /// they're not guaranteed to remain valid.
    pub unsafe fn locate_protocol<'boot, Protocol: proto::Protocol<'boot>>(
        &'boot self,
    ) -> Result<Option<Protocol>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = Protocol::GUID;
        let lp = self
            .interface()
            .locate_protocol
            .ok_or(EfiStatus::UNSUPPORTED)?;
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

    /// Exclusively open a protocol on `handle` if it exists,
    /// returning a [`Scope`] over the requested protocol.
    ///
    /// The [`Scope`] ensues the Protocol is closed whe it goes out of scope.
    ///
    /// If the [`Scope`] is leaked, you will not be able to open this protocol
    /// again, but is safe.
    ///
    /// # Warning
    ///
    /// This will cause firmware to attempt to **stop** any drivers
    /// currently using this protocol, if they support doing so.
    ///
    /// This means, for example,
    /// if you have a system with one serial port,
    /// which the user is using to interact, and you exclusively open that port,
    /// the user can no longer interact with the system.
    /// The same applies for graphical devices and
    /// [`GraphicsOutput`][crate::proto::graphics::GraphicsOutput].
    pub fn open_protocol<'boot, Proto: proto::Protocol<'boot>>(
        &'boot self,
        handle: EfiHandle,
    ) -> Result<Option<Scope<Proto>>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = Proto::GUID;
        let op = self
            .interface()
            .open_protocol
            .ok_or(EfiStatus::UNSUPPORTED)?;
        let agent = get_image_handle().expect("UEFI Image Handle was null in open_protocol");

        // Safety: Construction ensures safety. Statically verified arguments.
        let ret = unsafe {
            (op)(
                handle,
                &mut guid,
                &mut out,
                agent,
                EfiHandle(null_mut()),
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
                    None,
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
    pub fn close_protocol<'boot, Proto: proto::Protocol<'boot>>(
        &self,
        handle: EfiHandle,
        agent: EfiHandle,
        controller: Option<EfiHandle>,
    ) -> Result<()> {
        let mut guid = Proto::GUID;
        let cp = self
            .interface()
            .close_protocol
            .ok_or(EfiStatus::UNSUPPORTED)?;

        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe {
            (cp)(
                handle,
                &mut guid,
                agent,
                controller.unwrap_or(EfiHandle(null_mut())),
            )
        }
        .into()
    }

    /// Install an instance of [proto::Protocol] on `handle`
    pub fn install_protocol<'boot, Proto: proto::Protocol<'boot>>(
        &self,
        handle: EfiHandle,
        interface: &'static mut Proto::Raw,
    ) -> Result<()> {
        // Safety:
        // `interface` being a static mut reference guarantees validity and lifetime.
        unsafe { self.install_protocol_ptr::<Proto>(handle, interface) }
    }

    /// Install a `Protocol` on `handle`
    ///
    /// # Safety
    ///
    /// - Pointer must be a valid instance of [proto::Protocol]
    /// - Pointer must live long enough
    pub unsafe fn install_protocol_ptr<'boot, Proto: proto::Protocol<'boot>>(
        &self,
        handle: EfiHandle,
        interface: *mut Proto::Raw,
    ) -> Result<()> {
        let mut guid = Proto::GUID;
        let mut h = handle;
        let ipi = self
            .interface()
            .install_protocol_interface
            .ok_or(EfiStatus::UNSUPPORTED)?;

        (ipi)(&mut h, &mut guid, 0, interface as *mut u8).into()
    }

    /// Query `handle` to determine if it supports `Protocol`
    ///
    /// If no protocol is found, [`Ok(None)`] is returned.
    ///
    /// # Note
    ///
    /// This is deprecated by UEFI, and [`BootServices::open_protocol`] should
    /// be used in all new applications and drivers.
    ///
    /// This is because firmware is not notified that this protocol is in use,
    /// and there is not necessarily a guarantee they remain valid.
    ///
    /// # Safety
    ///
    /// - The returned Protocol must not already be in use
    // #[deprecated(note = "`BootServices::handle_protocol` is deprecated by UEFI")]
    pub unsafe fn handle_protocol<'boot, Protocol: proto::Protocol<'boot>>(
        &'boot self,
        handle: EfiHandle,
    ) -> Result<Option<Protocol>> {
        fn inner(
            guid: &Guid,
            handle: EfiHandle,
            hp: HandleProtocolFn,
            p_name: &'static str,
        ) -> Result<Option<NonNull<u8>>> {
            let mut out: *mut u8 = null_mut();

            // Safety: Arguments are statically valid for this call.
            let ret = unsafe { (hp)(handle, guid, &mut out) };

            if ret.is_success() {
                assert!(
                    !out.is_null(),
                    "UEFI handle_protocol returned success, but the protocol was null. \
                    The Protocol was \"{}\" with GUID `{}`",
                    p_name,
                    guid.to_uuid()
                );
                // Safety:
                // - Success means `out` is valid
                // - We assert its not null just in case
                unsafe { Ok(Some(NonNull::new_unchecked(out))) }
            } else if ret == EfiStatus::UNSUPPORTED {
                Ok(None)
            } else {
                Err(UefiError::new(ret))
            }
        }

        let mut out: *mut u8 = null_mut();
        let guid = Protocol::GUID;
        let hp = self
            .interface()
            .handle_protocol
            .ok_or(EfiStatus::UNSUPPORTED)?;

        let ret = inner(&guid, handle, hp, Protocol::NAME);

        match ret {
            Ok(Some(ret)) => {
                // Safety: `ret` is NonNull and from firmware
                unsafe { Ok(Some(Protocol::from_raw(ret.as_ptr() as *mut Protocol::Raw))) }
            }

            Ok(None) => Err(EfiStatus::UNSUPPORTED.into()),

            Err(e) => Err(e),
        }
    }
}

/// Image Services
impl<'table> BootServices<'table> {
    /// Exit the image represented by `handle` with `status`
    pub fn exit(&self, handle: EfiHandle, status: EfiStatus) -> Result<()> {
        let e = self.interface().exit.ok_or(EfiStatus::UNSUPPORTED)?;
        // Safety: Construction ensures safety
        unsafe { (e)(handle, status, 0, null_mut()) }.into()
    }

    /// Load an image from memory `src`, returning its handle.
    ///
    /// `parent` should be your image handle, as your will be th parent of this
    /// new image.
    ///
    /// If the image was from a device, you should set `devpath` to the
    /// [`DevicePath`] for the image on that device.
    ///
    /// Note that this will return [Ok] on a [`EfiStatus::SECURITY_VIOLATION`].
    ///
    /// You will need to handle that case in [`BootServices::start_image`]
    pub fn load_image(
        &self,
        parent: EfiHandle,
        devpath: Option<&DevicePath>,
        src: &[u8],
    ) -> Result<EfiHandle> {
        let mut out = EfiHandle(null_mut());

        // Safety: Statically correct for this operation
        // - policy is always false
        // - Devpath is statically valid or null
        // - parent is statically valid
        // - Source buffer and its size are valid
        unsafe {
            let devpath = devpath.map(|d| d.as_ptr()).unwrap_or(null_mut());
            self.load_image_impl(false, devpath, parent, src.as_ptr() as *mut u8, src.len())
        }
    }

    /// Load an image specified by [`DevicePath`]
    /// , returning its handle.
    ///
    /// `parent` should be your image handle, as your will be th parent of this
    /// new image.
    ///
    /// If the image was from a device, you should set `devpath` to the
    /// [`DevicePath`] for the image on that device.
    ///
    /// Note that this will return [Ok] on a [`EfiStatus::SECURITY_VIOLATION`].
    ///
    /// You will need to handle that case in [`BootServices::start_image`]
    pub fn load_image_fs(&self, parent: EfiHandle, devpath: &DevicePath) -> Result<EfiHandle> {
        let mut out = EfiHandle(null_mut());

        // Safety: Statically correct for this operation
        // - policy is always false
        // - Devpath is statically valid
        // - parent is statically valid
        // - Source buffer and its size are always null
        unsafe {
            let devpath = devpath.as_ptr();
            self.load_image_impl(false, devpath, parent, null_mut(), 0)
        }
    }

    /// Start an image loaded from [`LoadedImage`][loaded] earlier loaded image
    ///
    /// # Safety
    ///
    /// Because UEFI is not an OS, this is equivalent to a FFI call.
    /// In particular, an evil image could corrupt the system table and
    /// protocols we depend on as it pleases.
    ///
    /// This is only safe to call if you trust the application not to destroy
    /// the address space or be evil
    ///
    /// Take care not to run untrusted applications for other security reasons
    /// too.
    ///
    /// [loaded]: crate::proto::loaded_image::LoadedImage
    pub unsafe fn start_image(&self, handle: EfiHandle) -> Result<()> {
        let si = self.interface().start_image.ok_or(EfiStatus::UNSUPPORTED)?;
        // Safety: Construction ensures safety. Statically verified arguments.
        // FIXME: We are responsible for freeing ExitData
        let mut size = 0;
        unsafe { (si)(handle, &mut size, null_mut()).into() }
    }

    /// Unload an earlier loaded image
    pub fn unload_image(&self, handle: EfiHandle) -> Result<()> {
        let ui = self
            .interface()
            .unload_image
            .ok_or(EfiStatus::UNSUPPORTED)?;
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe { (ui)(handle).into() }
    }
}

/// Miscellaneous
impl<'table> BootServices<'table> {
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
        let s = self.interface().stall.ok_or(EfiStatus::UNSUPPORTED)?;

        // Safety: Construction ensures safety
        unsafe { (s)(time) }.into()
    }

    /// The next monotonic count
    pub fn next_monotonic_count(&self) -> Result<u64> {
        let mut out = 0;
        let gn = self
            .interface()
            .get_next_monotonic_count
            .ok_or(EfiStatus::UNSUPPORTED)?;

        // Safety: Construction ensures safety
        let ret = unsafe { (gn)(&mut out) };
        if ret.is_success() {
            return Ok(out);
        }
        Err(UefiError::new(ret))
    }

    /// Set the watchdog timer. [`None`] disables the timer.
    pub fn set_watchdog(&self, timeout: Option<Duration>) -> Result<()> {
        let timeout = timeout.unwrap_or_default();
        let swt = self
            .interface()
            .set_watchdog_timer
            .ok_or(EfiStatus::UNSUPPORTED)?;

        let secs = match timeout
            .as_secs()
            .try_into()
            .map_err(|_| EfiStatus::INVALID_PARAMETER)
        {
            Ok(t) => t,
            Err(e) => return e.into(),
        };
        // Safety: Construction ensures safety. Statically verified arguments.
        unsafe { (swt)(secs, 0x10000, 0, null_mut()) }.into()
    }
}

/// Memory Allocation Services
impl<'table> BootServices<'table> {
    /// Allocate `size` bytes of memory from pool of type `ty`.
    /// Allocations are 8 byte aligned.
    ///
    /// This will fail if `ty` is [MemoryType::RESERVED]
    #[inline]
    pub fn allocate_pool(&self, ty: MemoryType, size: usize) -> Result<NonNull<u8>> {
        if ty == MemoryType::RESERVED {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }
        let mut out: *mut u8 = null_mut();

        let ap = self
            .interface()
            .allocate_pool
            .ok_or(EfiStatus::UNSUPPORTED)?;

        // Safety: Always valid for these arguments
        // - `ap` checked above
        // - memory errors wont happen from invalid arguments
        // - we never provide invalid pointers
        let ret = unsafe { (ap)(ty, size, &mut out) };

        if ret.is_success() {
            assert!(!out.is_null(), "UEFI Allocator returned successful null");
            // Safety: assert
            unsafe { Ok(NonNull::new_unchecked(out)) }
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Allocate [`size_of::<T>()`] bytes from pool of type `ty`.
    /// Allocations are 8 byte aligned.
    ///
    /// This is a convenience method around [`BootServices::allocate_pool`]
    /// and casting the pointer manually.
    ///
    /// # Safety
    ///
    /// Unlike [`BootServices::allocate_pool`], this is unsafe,
    /// because `T` may not be 8-bytes aligned
    #[inline]
    pub unsafe fn allocate_pool_ty<T>(&self, ty: MemoryType) -> Result<NonNull<T>> {
        self.allocate_pool(ty, size_of::<T>()).map(|n| n.cast())
    }

    /// The same as [`allocate_pool_ty`][alloc_ty], but allocates `len`
    /// *elements* of `T`.
    ///
    /// # Safety
    ///
    /// See [`allocate_pool_ty`][alloc_ty]
    ///
    /// [alloc_ty]: BootServices::allocate_pool_ty
    #[inline]
    pub unsafe fn allocate_pool_ty_array<T>(
        &self,
        ty: MemoryType,
        len: usize,
    ) -> Result<NonNull<T>> {
        self.allocate_pool(ty, len * size_of::<T>())
            .map(|n| n.cast())
    }

    /// Free memory allocated by [BootServices::allocate_pool]
    ///
    /// # Safety
    ///
    /// - Must have been allocated by [BootServices::allocate_pool]
    /// - Must be non-null
    #[inline]
    pub unsafe fn free_pool(&self, memory: *mut u8) -> Result<()> {
        let fp = self.interface().free_pool.ok_or(EfiStatus::UNSUPPORTED)?;
        (fp)(memory).into()
    }
}

/// Event/Timer/Task Priority
impl<'table> BootServices<'table> {}

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
// safety guarantees of passing it to ExitBootServices.
//
// It is also important that the lifetimes involved stay within their
// respective structures, that the lifetime of the SystemTable is not used
// for data from BootServices.
// That way we can potentially statically prevent incorrect ExitBootServices
// calls, without invalidating RuntimeServices?
// Existing RuntimeServices and pointers might become invalid though?
//
// Defining lifetimes this way should be fine either way though
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

// Internal, all
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

// Internal, all
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

/// Available during Boot Services
impl SystemTable<Boot> {
    /// String identifying the vendor
    pub fn firmware_vendor(&self) -> String {
        let p = self.table().firmware_vendor as *mut u16;
        if p.is_null() {
            return String::new();
        }
        // Safety: always valid
        unsafe { UefiStr::from_ptr(p) }.into_string()
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

    /// Output on stdout.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn stdout(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_out;
        assert!(!ptr.is_null(), "con_out handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr) }
    }

    /// Output on stderr.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn stderr(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_err;
        assert!(!ptr.is_null(), "std_err handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr) }
    }

    /// Reference to the UEFI Boot services.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn boot(&self) -> BootServices<'_> {
        let ptr = self.table().boot_services;
        assert!(!ptr.is_null(), "boot_services handle was null");
        // Safety: Construction ensures safety.
        unsafe { BootServices::new(ptr) }
    }

    /// Iterator over UEFI Configuration tables
    pub fn config_tables(&self) -> impl Iterator<Item = config::GenericConfig<'_>> + '_ {
        let data = self.table().configuration_table;
        let len = self.table().number_of_table_entries;
        assert!(!data.is_null(), "UEFI Configuration table pointer was null");

        // Safety: The pointer is valid for this many elements according
        // to the UEFI spec
        // The returned lifetime will be tied to `self`, which is valid.
        let tables = unsafe { from_raw_parts(data, len).iter().copied() };

        tables.map(config::GenericConfig::new)
    }

    /// Get the configuration table specified by `T`, or [`None`]
    pub fn config_table<'tbl, T: config::ConfigTable<'tbl>>(&'tbl self) -> Option<T::Out<'tbl>>
    where
        Self: 'tbl,
    {
        self.config_tables()
            .find(|t| t.guid() == T::GUID)
            .and_then(|t| t.as_table::<T>())
    }
}
