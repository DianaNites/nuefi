//! Raw UEFI data types
use core::{
    mem::size_of,
    ptr::{null_mut, NonNull},
};

use crate::{
    error::{EfiStatus, Result},
    proto::{
        self,
        console::raw::{RawSimpleTextInput, RawSimpleTextOutput},
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
    pub const fn new(major: u16, minor: u16) -> Self {
        Revision(((major as u32) << 16) | minor as u32)
    }

    pub const fn major(self) -> u32 {
        self.0 >> 16
    }

    pub const fn minor(self) -> u32 {
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
        assert!(!table.is_null(), "Table Header ({sig}) was null");
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
            (len as usize)
                .checked_sub(size_of::<Header>())
                .ok_or(EfiStatus::BUFFER_TOO_SMALL)?,
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
        loop {}
        Header::validate(
            header.runtime_services as *mut Header,
            RawRuntimeServices::SIGNATURE,
        )?;
        Ok(())
    }

    fn to_bytes(&self) -> &[u8] {
        // Safety: `self` is valid by definition
        // Lifetime is bound to self
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>())
        };
        bytes
    }

    /// Mock instance of [`RawSystemTable`]
    #[doc(hidden)]
    #[allow(unreachable_code, unused_mut)]
    pub unsafe fn mock() -> Self {
        const MOCK_VENDOR: &[u8] = b"Mock Vendor";
        let mut mock_vendor = [0u8; MOCK_VENDOR.len()];
        // let len = MOCK_VENDOR.chars().fold(0, |a, e| a + e.len_utf16());
        // firmware_vendor.encode_utf16();

        const MOCK_VENDOR_16: *const u16 = null_mut();
        const MOCK_HEADER: Header = Header {
            signature: RawSystemTable::SIGNATURE,
            revision: Revision::new(2, 70),
            size: size_of::<RawSystemTable>() as u32,
            crc32: 0,
            reserved: 0,
        };
        const MOCK_SYSTEM: RawSystemTable = RawSystemTable {
            header: MOCK_HEADER,
            firmware_vendor: MOCK_VENDOR_16,
            firmware_revision: 69420,
            console_in_handle: EfiHandle(null_mut()),
            con_in: null_mut(),
            console_out_handle: EfiHandle(null_mut()),
            con_out: null_mut(),
            standard_error_handle: EfiHandle(null_mut()),
            std_err: null_mut(),
            runtime_services: null_mut(),
            boot_services: null_mut(),
            number_of_table_entries: 0,
            configuration_table: null_mut(),
        };

        const MOCK_BOOT: RawBootServices = RawBootServices::mock();

        let mut s = MOCK_SYSTEM;

        s.header.crc32 = {
            let mut digest = CRC.digest();
            digest.update(s.to_bytes());
            digest.finalize()
        };
        s
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

    const fn mock() -> Self {
        const MOCK_HEADER: Header = Header {
            signature: RawBootServices::SIGNATURE,
            revision: Revision::new(2, 70),
            size: size_of::<RawBootServices>() as u32,
            crc32: 0,
            reserved: 0,
        };
        let b = [0u8; size_of::<Self>()];
        // Safety:
        let mut t: RawBootServices = unsafe { core::mem::transmute::<_, _>(b) };
        t.header = MOCK_HEADER;
        return t;
        #[cfg(no)]
        RawBootServices {
            header: MOCK_HEADER,
            raise_tpl: null_mut(),
            restore_tpl: null_mut(),
            allocate_pages: null_mut(),
            free_pages: null_mut(),
            get_memory_map: null_mut(),
            allocate_pool: null_mut(),
            free_pool: null_mut(),
            create_event: null_mut(),
            set_timer: null_mut(),
            wait_for_event: null_mut(),
            signal_event: null_mut(),
            close_event: null_mut(),
            check_event: null_mut(),
            install_protocol_interface: null_mut(),
            reinstall_protocol_interface: null_mut(),
            uninstall_protocol_interface: null_mut(),
            handle_protocol: null_mut(),
            reserved: null_mut(),
            register_protocol_notify: null_mut(),
            locate_handle: null_mut(),
            locate_device_path: null_mut(),
            install_configuration_table: null_mut(),
            load_image: null_mut(),
            start_image: null_mut(),
            exit: null_mut(),
            unload_image: null_mut(),
            exit_boot_services: null_mut(),
            get_next_monotonic_count: null_mut(),
            stall: null_mut(),
            set_watchdog_timer: null_mut(),
            connect_controller: null_mut(),
            disconnect_controller: null_mut(),
            open_protocol: null_mut(),
            close_protocol: null_mut(),
            open_protocol_information: null_mut(),
            protocols_per_handle: null_mut(),
            locate_handle_buffer: null_mut(),
            locate_protocol: null_mut(),
            install_multiple_protocol_interfaces: null_mut(),
            uninstall_multiple_protocol_interfaces: null_mut(),
            calculate_crc32: null_mut(),
            copy_mem: null_mut(),
            set_mem: null_mut(),
            create_event_ex: null_mut(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision() {
        let rev = Revision::new(2, 70);
        assert_eq!(rev.major(), 2);
        assert_eq!(rev.minor(), 70);
    }
}
