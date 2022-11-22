//! UEFI Tables

use core::{marker::PhantomData, mem::size_of, ptr::null_mut, time::Duration};

use crate::{
    error::{EfiStatus, Result, UefiError},
    proto::{
        self,
        console::{RawSimpleTextInput, RawSimpleTextOutput, SimpleTextOutput},
        device_path::DevicePath,
        Scope,
    },
    util::interface,
    EfiHandle,
};

pub static CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

type Void = *mut [u8; 0];

/// UEFI Header Revision
///
/// This is a binary coded decimal.
///
/// The upper 16 bits are the major version
///
/// The lower 16 bits are the minor version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct Revision(u32);

impl Revision {
    pub fn major(self) -> u32 {
        self.0 >> 16
    }

    pub fn minor(self) -> u32 {
        self.0 as u16 as u32
    }
}

#[derive(Debug)]
#[repr(C)]
struct Header {
    /// Unique signature identifying the table
    signature: u64,

    /// UEFI Revision
    revision: Revision,

    /// Size of the entire table, including this header
    size: u32,

    /// 32-bit CRC for the table.
    /// This is set to 0 and computed for `size` bytes.
    crc32: u32,

    /// Reserved field. 0.
    reserved: u32,
}

impl Header {
    /// Validate the header
    ///
    /// # Safety
    ///
    /// - Must be called with a valid pointed to a UEFI table
    unsafe fn validate(table: *mut Self, sig: u64) -> Result<()> {
        let header = &*table;
        let expected = header.crc32;
        let len = header.size;
        // Calculate the CRC
        let mut digest = CRC.digest();
        digest.update(&header.signature.to_ne_bytes());
        digest.update(&header.revision.0.to_ne_bytes());
        digest.update(&header.size.to_ne_bytes());
        digest.update(&0u32.to_ne_bytes());
        digest.update(&header.reserved.to_ne_bytes());
        // Calculate the remaining table, header digested above.
        let bytes = core::slice::from_raw_parts(
            table.cast::<u8>().add(size_of::<Header>()),
            len as usize - size_of::<Header>(),
        );
        digest.update(bytes);
        if expected != digest.finalize() {
            return EfiStatus::CRC_ERROR.into();
        }
        if !(header.revision.major() == 2 && header.revision.minor() >= 70) {
            return EfiStatus::UNSUPPORTED.into();
        }
        if header.signature != sig {
            return EfiStatus::INVALID_PARAMETER.into();
        }
        Ok(())
    }
}

/// The EFI system table.
///
/// After a call to ExitBootServices, some parts of this may become invalid.
#[derive(Debug)]
#[repr(C)]
pub struct RawSystemTable {
    /// Table header, always valid
    header: Header,

    /// Firmware vendor, always valid
    ///
    /// Null terminated UCS-2 string
    firmware_vendor: *const u16,

    /// Firmware revision, always valid
    ///
    /// Firmware vendor specific version value
    firmware_revision: u32,

    ///
    console_in_handle: EfiHandle,

    ///
    con_in: *mut RawSimpleTextInput,

    ///
    console_out_handle: EfiHandle,

    ///
    con_out: *mut RawSimpleTextOutput,

    ///
    standard_error_handle: EfiHandle,

    ///
    std_err: *mut RawSimpleTextOutput,

    /// Runtime services table, always valid
    runtime_services: *mut RawRuntimeServices,

    /// Boot services table
    boot_services: *mut RawBootServices,

    /// Number of entries, always valid
    number_of_table_entries: usize,

    /// Configuration table, always valid
    configuration_table: Void, // EFI_CONFIGURATION_TABLE
}

impl RawSystemTable {
    const SIGNATURE: u64 = 0x5453595320494249;

    /// Validate the table
    ///
    /// Validation fails if CRC validation fails, or the UEFI revision is
    /// unsupported
    ///
    /// # Safety
    ///
    /// - Must be a valid pointer
    /// - Must only be called before running user code.
    pub(crate) unsafe fn validate(this: *mut Self) -> Result<()> {
        // Safety: Pointer to first C struct member
        Header::validate(this as *mut Header, Self::SIGNATURE)?;
        let header = &(*this);
        Header::validate(
            header.boot_services as *mut Header,
            RawBootServices::SIGNATURE,
        )?;
        Header::validate(
            header.runtime_services as *mut Header,
            RawRuntimeServices::SIGNATURE,
        )?;
        Ok(())
    }
}

// #[derive(Debug)]
#[repr(C)]
pub struct RawBootServices {
    /// Table header
    header: Header,

    // Task priority
    raise_tpl: Void,
    restore_tpl: Void,

    // Memory
    allocate_pages: unsafe extern "efiapi" fn(
        ty: crate::mem::AllocateType,
        mem_ty: crate::mem::MemoryType,
        pages: usize,
        memory: *mut crate::mem::PhysicalAddress,
    ) -> EfiStatus,
    free_pages: unsafe extern "efiapi" fn(
        //
        memory: crate::mem::PhysicalAddress,
        pages: usize,
    ) -> EfiStatus,
    get_memory_map: unsafe extern "efiapi" fn(
        map_size: *mut usize,
        map: *mut crate::mem::MemoryDescriptor,
        key: *mut usize,
        entry_size: *mut usize,
        entry_version: *mut u32,
    ) -> EfiStatus,
    allocate_pool: unsafe extern "efiapi" fn(
        mem_ty: crate::mem::MemoryType,
        size: usize,
        out: *mut *mut u8,
    ) -> EfiStatus,
    free_pool: unsafe extern "efiapi" fn(mem: *mut u8) -> EfiStatus,

    // Timers/Events
    create_event: Void,
    set_timer: Void,
    wait_for_event: Void,
    signal_event: Void,
    close_event: Void,
    check_event: Void,

    // Protocols
    install_protocol_interface: unsafe extern "efiapi" fn(
        handle: *mut EfiHandle,
        guid: *mut proto::Guid,
        interface_ty: u32,
        interface: *mut u8,
    ) -> EfiStatus,
    reinstall_protocol_interface: Void,
    uninstall_protocol_interface: Void,
    handle_protocol: Void,
    reserved: Void,
    register_protocol_notify: Void,
    locate_handle: Void,
    locate_device_path: Void,
    install_configuration_table: Void,

    // Images
    load_image: unsafe extern "efiapi" fn(
        policy: bool,
        parent: EfiHandle,
        path: *mut DevicePath,
        source: *mut u8,
        source_size: usize,
        out: *mut EfiHandle,
    ) -> EfiStatus,
    start_image: unsafe extern "efiapi" fn(
        //
        handle: EfiHandle,
        exit_size: *mut usize,
        exit: *mut *mut u8,
    ) -> EfiStatus,
    exit: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        status: EfiStatus,
        data_size: usize,
        data: proto::Str16,
    ) -> EfiStatus,
    unload_image: unsafe extern "efiapi" fn(handle: EfiHandle) -> EfiStatus,
    exit_boot_services: unsafe extern "efiapi" fn(handle: EfiHandle, key: usize) -> EfiStatus,

    // Misc
    get_next_monotonic_count: unsafe extern "efiapi" fn(count: *mut u64) -> EfiStatus,
    stall: unsafe extern "efiapi" fn(microseconds: usize) -> EfiStatus,
    set_watchdog_timer: unsafe extern "efiapi" fn(
        timeout: usize,
        code: u64,
        data_size: usize,
        data: proto::Str16,
    ) -> EfiStatus,

    // Drivers
    connect_controller: Void,
    disconnect_controller: Void,

    // Protocols again
    open_protocol: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        guid: *mut proto::Guid,
        out: *mut *mut u8,
        agent_handle: EfiHandle,
        controller_handle: EfiHandle,
        attributes: u32,
    ) -> EfiStatus,
    close_protocol: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        guid: *mut proto::Guid,
        agent_handle: EfiHandle,
        controller_handle: EfiHandle,
    ) -> EfiStatus,
    open_protocol_information: Void,

    // Library?
    protocols_per_handle: Void,
    locate_handle_buffer: Void,
    locate_protocol: unsafe extern "efiapi" fn(
        //
        guid: *mut proto::Guid,
        key: Void,
        out: *mut *mut u8,
    ) -> EfiStatus,
    install_multiple_protocol_interfaces: Void,
    uninstall_multiple_protocol_interfaces: Void,

    // Useless CRC
    calculate_crc32: Void,

    // Misc again
    copy_mem: Void,
    set_mem: Void,
    create_event_ex: Void,
}

impl RawBootServices {
    const SIGNATURE: u64 = 0x56524553544f4f42;
}

interface!(
    /// The UEFI Boot Services
    BootServices(RawBootServices),
);

impl<'table> BootServices<'table> {
    /// Exit the image represented by `handle` with `status`
    pub fn exit(&self, handle: EfiHandle, status: EfiStatus) -> Result<()> {
        unsafe { (self.interface().exit)(handle, status, 0, null_mut()) }.into()
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
        unsafe { (self.interface().stall)(time) }.into()
    }

    /// The next monotonic count
    pub fn next_monotonic_count(&self) -> Result<u64> {
        let mut out = 0;
        let ret = unsafe { (self.interface().get_next_monotonic_count)(&mut out) };
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
        unsafe { (self.interface().set_watchdog_timer)(secs, 0x10000, 0, null_mut()) }.into()
    }

    /// Allocate `size` bytes of memory from pool of type `ty`
    pub fn allocate_pool(&self, ty: crate::mem::MemoryType, size: usize) -> Result<*mut u8> {
        let mut out: *mut u8 = null_mut();
        let ret = unsafe { (self.interface().allocate_pool)(ty, size, &mut out) };
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
    /// Must have been allocated by [BootServices::allocate_pool]
    pub unsafe fn free_pool(&self, memory: *mut u8) -> Result<()> {
        (self.interface().free_pool)(memory).into()
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
    pub fn locate_protocol<'boot, T: proto::Protocol<'boot>>(&'boot self) -> Result<Option<T>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = T::GUID;
        let ret = unsafe { (self.interface().locate_protocol)(&mut guid, null_mut(), &mut out) };
        if ret.is_success() {
            unsafe { Ok(Some(T::from_raw(out))) }
        } else if ret == EfiStatus::NOT_FOUND {
            Ok(None)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Open the protocol on `handle`, if it exists.
    ///
    /// The protocol is opened in Exclusive mode
    pub fn open_protocol<'boot, T: proto::Protocol<'boot>>(
        &'boot self,
        handle: EfiHandle,
        agent: EfiHandle,
        controller: Option<EfiHandle>,
    ) -> Result<Option<Scope<T>>> {
        let mut out: *mut u8 = null_mut();
        let mut guid = T::GUID;
        let ret = unsafe {
            (self.interface().open_protocol)(
                handle,
                &mut guid,
                &mut out,
                agent,
                controller.unwrap_or(EfiHandle(null_mut())),
                0x20,
            )
        };
        if ret.is_success() {
            unsafe {
                Ok(Some(Scope::new(
                    T::from_raw(out),
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

    /// Close the [Protocol] on `handle`
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
        unsafe {
            (self.interface().close_protocol)(
                handle,
                &mut guid,
                agent,
                controller.unwrap_or(EfiHandle(null_mut())),
            )
        }
        .into()
    }

    /// Load an image from memory `src`, returning its handle.
    pub fn load_image(&self, parent: EfiHandle, src: &[u8]) -> Result<EfiHandle> {
        let mut out = EfiHandle(null_mut());
        let ret = unsafe {
            (self.interface().load_image)(
                false,
                parent,
                // TODO: Provide fake device path
                null_mut(),
                // UEFI pls do not modify us.
                src.as_ptr() as *mut _,
                src.len(),
                &mut out,
            )
        };

        if ret.is_success() {
            assert_ne!(out, EfiHandle(null_mut()));
            Ok(out)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Unload an earlier loaded image
    pub fn start_image(&self, handle: EfiHandle) -> Result<()> {
        unsafe { (self.interface().start_image)(handle, &mut 0, null_mut()).into() }
    }

    /// Unload an earlier loaded image
    pub fn unload_image(&self, handle: EfiHandle) -> Result<()> {
        unsafe { (self.interface().unload_image)(handle).into() }
    }

    /// Install a `Protocol` on `handle`
    pub fn install_protocol<'a, T: proto::Protocol<'a>>(
        &self,
        handle: EfiHandle,
        interface: &mut T,
    ) -> Result<()> {
        let mut guid = T::GUID;
        let mut h = handle;
        unsafe {
            (self.interface().install_protocol_interface)(
                &mut h,
                &mut guid,
                0,
                interface as *mut _ as *mut u8,
            )
            .into()
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RawRuntimeServices {
    /// Table header
    header: Header,
}

impl RawRuntimeServices {
    const SIGNATURE: u64 = 0x56524553544e5552;
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

/// Type marker for [`SystemTable`] representing we dont know if
/// ExitBootServices has been called
struct Unknown;

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
            // Safety
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
    pub(crate) fn as_runtime(&self) -> Option<SystemTable<Boot>> {
        if !self.table().boot_services.is_null() {
            // Safety
            // - Above check verifies ExitBootServices has not been called.
            Some(unsafe { SystemTable::new(self.table) })
        } else {
            None
        }
    }
}

impl SystemTable<Boot> {
    /// String identifying the vendor
    pub fn firmware_vendor(&self) -> &str {
        ""
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
        unsafe { SimpleTextOutput::new(self.table().con_out) }
    }

    /// Output on stderr
    pub fn stderr(&self) -> SimpleTextOutput<'_> {
        unsafe { SimpleTextOutput::new(self.table().std_err) }
    }

    /// Reference to the UEFI Boot services.
    pub fn boot(&self) -> BootServices<'_> {
        unsafe { BootServices::new(self.table().boot_services) }
    }
}
