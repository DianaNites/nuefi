//! Raw UEFI data types
use core::mem::size_of;

use crate::{
    error::{EfiStatus, Result},
    proto::{
        self,
        console::{RawSimpleTextInput, RawSimpleTextOutput},
        device_path::DevicePath,
    },
    EfiHandle,
};

pub static CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

/// UEFI Header Revision
///
/// This is a binary coded decimal.
///
/// The upper 16 bits are the major version
///
/// The lower 16 bits are the minor version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Revision(u32);

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
pub struct Header {
    /// Unique signature identifying the table
    pub signature: u64,

    /// UEFI Revision
    pub revision: Revision,

    /// Size of the entire table, including this header
    pub size: u32,

    /// 32-bit CRC for the table.
    /// This is set to 0 and computed for `size` bytes.
    pub crc32: u32,

    /// Reserved field. 0.
    pub reserved: u32,
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
    pub header: Header,

    /// Firmware vendor, always valid
    ///
    /// Null terminated UCS-2 string
    pub firmware_vendor: *const u16,

    /// Firmware revision, always valid
    ///
    /// Firmware vendor specific version value
    pub firmware_revision: u32,

    ///
    pub console_in_handle: EfiHandle,

    ///
    pub con_in: *mut RawSimpleTextInput,

    ///
    pub console_out_handle: EfiHandle,

    ///
    pub con_out: *mut RawSimpleTextOutput,

    ///
    pub standard_error_handle: EfiHandle,

    ///
    pub std_err: *mut RawSimpleTextOutput,

    /// Runtime services table, always valid
    pub runtime_services: *mut RawRuntimeServices,

    /// Boot services table
    pub boot_services: *mut RawBootServices,

    /// Number of entries, always valid
    pub number_of_table_entries: usize,

    /// Configuration table, always valid
    pub configuration_table: *mut u8, // EFI_CONFIGURATION_TABLE
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
    pub header: Header,

    // Task priority
    pub raise_tpl: *mut u8,
    pub restore_tpl: *mut u8,

    // Memory
    pub allocate_pages: unsafe extern "efiapi" fn(
        ty: crate::mem::AllocateType,
        mem_ty: crate::mem::MemoryType,
        pages: usize,
        memory: *mut crate::mem::PhysicalAddress,
    ) -> EfiStatus,
    pub free_pages: unsafe extern "efiapi" fn(
        //
        memory: crate::mem::PhysicalAddress,
        pages: usize,
    ) -> EfiStatus,
    pub get_memory_map: unsafe extern "efiapi" fn(
        map_size: *mut usize,
        map: *mut crate::mem::MemoryDescriptor,
        key: *mut usize,
        entry_size: *mut usize,
        entry_version: *mut u32,
    ) -> EfiStatus,
    pub allocate_pool: unsafe extern "efiapi" fn(
        mem_ty: crate::mem::MemoryType,
        size: usize,
        out: *mut *mut u8,
    ) -> EfiStatus,
    pub free_pool: unsafe extern "efiapi" fn(mem: *mut u8) -> EfiStatus,

    // Timers/Events
    pub create_event: *mut u8,
    pub set_timer: *mut u8,
    pub wait_for_event: *mut u8,
    pub signal_event: *mut u8,
    pub close_event: *mut u8,
    pub check_event: *mut u8,

    // Protocols
    pub install_protocol_interface: unsafe extern "efiapi" fn(
        handle: *mut EfiHandle,
        guid: *mut proto::Guid,
        interface_ty: u32,
        interface: *mut u8,
    ) -> EfiStatus,
    pub reinstall_protocol_interface: *mut u8,
    pub uninstall_protocol_interface: *mut u8,
    pub handle_protocol: *mut u8,
    pub reserved: *mut u8,
    pub register_protocol_notify: *mut u8,
    pub locate_handle: *mut u8,
    pub locate_device_path: *mut u8,
    pub install_configuration_table: *mut u8,

    // Images
    pub load_image: unsafe extern "efiapi" fn(
        policy: bool,
        parent: EfiHandle,
        path: *mut DevicePath,
        source: *mut u8,
        source_size: usize,
        out: *mut EfiHandle,
    ) -> EfiStatus,
    pub start_image: unsafe extern "efiapi" fn(
        //
        handle: EfiHandle,
        exit_size: *mut usize,
        exit: *mut *mut u8,
    ) -> EfiStatus,
    pub exit: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        status: EfiStatus,
        data_size: usize,
        data: proto::Str16,
    ) -> EfiStatus,
    pub unload_image: unsafe extern "efiapi" fn(handle: EfiHandle) -> EfiStatus,
    pub exit_boot_services: unsafe extern "efiapi" fn(handle: EfiHandle, key: usize) -> EfiStatus,

    // Misc
    pub get_next_monotonic_count: unsafe extern "efiapi" fn(count: *mut u64) -> EfiStatus,
    pub stall: unsafe extern "efiapi" fn(microseconds: usize) -> EfiStatus,
    pub set_watchdog_timer: unsafe extern "efiapi" fn(
        timeout: usize,
        code: u64,
        data_size: usize,
        data: proto::Str16,
    ) -> EfiStatus,

    // Drivers
    pub connect_controller: *mut u8,
    pub disconnect_controller: *mut u8,

    // Protocols again
    pub open_protocol: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        guid: *mut proto::Guid,
        out: *mut *mut u8,
        agent_handle: EfiHandle,
        controller_handle: EfiHandle,
        attributes: u32,
    ) -> EfiStatus,
    pub close_protocol: unsafe extern "efiapi" fn(
        handle: EfiHandle,
        guid: *mut proto::Guid,
        agent_handle: EfiHandle,
        controller_handle: EfiHandle,
    ) -> EfiStatus,
    pub open_protocol_information: *mut u8,

    // Library?
    pub protocols_per_handle: *mut u8,
    pub locate_handle_buffer: *mut u8,
    pub locate_protocol: unsafe extern "efiapi" fn(
        //
        guid: *mut proto::Guid,
        key: *mut u8,
        out: *mut *mut u8,
    ) -> EfiStatus,
    pub install_multiple_protocol_interfaces: *mut u8,
    pub uninstall_multiple_protocol_interfaces: *mut u8,

    // Useless CRC
    pub calculate_crc32: *mut u8,

    // Misc again
    pub copy_mem: *mut u8,
    pub set_mem: *mut u8,
    pub create_event_ex: *mut u8,
}

impl RawBootServices {
    const SIGNATURE: u64 = 0x56524553544f4f42;
}

#[derive(Debug)]
#[repr(C)]
pub struct RawRuntimeServices {
    /// Table header
    pub header: Header,
}

impl RawRuntimeServices {
    const SIGNATURE: u64 = 0x56524553544e5552;
}
