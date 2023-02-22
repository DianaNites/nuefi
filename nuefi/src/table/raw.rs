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
        graphics::{raw::RawGraphicsOutput, GraphicsOutput},
        Protocol,
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
    /// Create a new revision for `major.minor`
    pub const fn new(major: u16, minor: u16) -> Self {
        Revision(((major as u32) << 16) | minor as u32)
    }

    /// The major part of the revision
    pub const fn major(self) -> u32 {
        self.0 >> 16
    }

    /// The minor part of the revision
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
    // `376` is the biggest table size we know about.
    // `352` is that minus `24`, the Header size
    fn to_rem_bytes(&self, ptr: *const u8, len: usize) -> ([u8; 352], usize) {
        union Buf {
            h: core::mem::ManuallyDrop<Header>,
            buf: [u8; 352],
        }

        let mut buf = [0u8; 352];
        let len = len.saturating_sub(size_of::<Header>());
        // Safety:
        unsafe {
            ptr.add(size_of::<Header>()).copy_to(buf.as_mut_ptr(), len);
            return (buf, len);
            // let ptr
            let bytes = core::slice::from_raw_parts(
                ptr.add(size_of::<Header>()),
                len.saturating_sub(size_of::<Header>()),
            );
            // FIXME: Alignment? Uninit Padding? Causes various hard to detect UB?
            // Miri is reporting problems here but only for RawSystemHeader?
            // It has uninit padding
            // The standard requires we read for `size` *bytes* though, not
            // fields, so we need a solution. fucking C.
            // Maybe its a stacked borrows/miri bug and not UB?
            // bytes
            (bytes.try_into().unwrap(), len)
        }
    }

    /// Validate the header
    ///
    /// # Safety
    ///
    /// - Must be called with a valid pointed to a UEFI table
    /// - `table` is implicitly trusted as valid/sensible where it is not
    ///   possible to verify.
    ///     - Broken/buggy UEFI implementations will be able to cause  the
    ///       following UB:
    ///         - // TODO: List UB
    ///         - Uninitialized padding readings from system tables
    unsafe fn validate(table: *const u8, sig: u64) -> Result<()> {
        assert!(!table.is_null(), "Table Header ({sig:#X}) was null");

        // Safety: `table` is non-null and trusted by firmware
        let header = &*(table as *const Self);
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
        let (bytes, len) = header.to_rem_bytes(table, len as usize);
        let bytes = &bytes[..len];

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

    /// Padding inherent in the layout.
    /// We rely on initialized data here for safety.
    ///
    /// This padding is only? on 64-bit because `EfiHandle` is a a pointer
    // pub _pad1: [u8; 4],

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
        // FIXME: Miri failure here. Actual uB or bug or TBD?
        #[cfg(not(miri))]
        {
            // Safety: Pointer to first C struct member
            Header::validate(this as *const u8, Self::SIGNATURE)?;

            let header = &(*this);

            Header::validate(
                header.boot_services as *const u8,
                RawBootServices::SIGNATURE,
            )?;
            Header::validate(
                header.runtime_services as *const u8,
                RawRuntimeServices::SIGNATURE,
            )?;
        }
        Ok(())
    }

    // FIXME: This
    fn to_bytes(&self) -> [u8; size_of::<Self>()] {
        #[cfg(no)]
        panic!(
            "\n\n\n\n\n\
RawSystemTable:
    align {} // 8
    size {} // 120

RawBootServices:
    align {} // 8
    size {} // 376

RawRuntimeServices:
    align {} // 8
    size {} // 24

Header:
    align {} // 8
    size {} // 24
        \n\n\n\n\n",
            core::mem::align_of::<Self>(),
            core::mem::size_of::<Self>(),
            core::mem::align_of::<RawBootServices>(),
            core::mem::size_of::<RawBootServices>(),
            core::mem::align_of::<RawRuntimeServices>(),
            core::mem::size_of::<RawRuntimeServices>(),
            core::mem::align_of::<Header>(),
            core::mem::size_of::<Header>(),
        );
        // #[cfg(no)]
        // Safety: `self` is valid by definition
        // Lifetime is bound to self
        unsafe {
            let b =
                core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>());
            // b.try_into().unwrap()
            [0u8; size_of::<Self>()]
        }
    }

    const fn new_mock(header: Header) -> Self {
        RawSystemTable {
            header,
            firmware_vendor: null_mut(),
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
            // _pad1: [0u8; 4],
            // _pad2: [0u8; 2],
        }
    }

    /// Mock instance of [`RawSystemTable`]
    #[doc(hidden)]
    pub unsafe fn mock() -> *mut Self {
        const MOCK_VENDOR: &str = "Mock Vendor";
        static mut BUF: &mut [u16] = &mut [0u16; MOCK_VENDOR.len() + 1];
        MOCK_VENDOR
            .encode_utf16()
            .enumerate()
            .for_each(|(i, f)| BUF[i] = f);

        const MOCK_HEADER: Header = Header {
            signature: RawSystemTable::SIGNATURE,
            revision: Revision::new(2, 70),
            size: size_of::<RawSystemTable>() as u32,
            crc32: 0,
            reserved: 0,
        };
        static mut MOCK_SYSTEM: RawSystemTable = RawSystemTable::new_mock(MOCK_HEADER);

        static mut MOCK_BOOT: YesSync<RawBootServices> = YesSync(RawBootServices::mock());
        static mut MOCK_RUN: YesSync<RawRuntimeServices> = YesSync(RawRuntimeServices::mock());
        static mut MOCK_OUT: YesSync<RawSimpleTextOutput> = YesSync(RawSimpleTextOutput::mock());
        static mut MOCK_GOP: YesSync<RawGraphicsOutput> = YesSync(RawGraphicsOutput::mock());

        // Safety: We only mock once, single threaded
        if MOCK_SYSTEM.header.crc32 == 0 {
            let mut s = &mut MOCK_SYSTEM;

            // Safety:
            // It is important for safety/miri that references not be created
            // slash that these pointers not be derived from them.
            s.boot_services = core::ptr::addr_of!(MOCK_BOOT.0) as *mut _;
            s.runtime_services = core::ptr::addr_of!(MOCK_RUN.0) as *mut _;
            s.con_out = core::ptr::addr_of!(MOCK_OUT.0) as *mut _;
            s.firmware_vendor = BUF.as_ptr();

            unsafe extern "efiapi" fn locate_protocol(
                guid: *mut proto::Guid,
                key: *mut u8,
                out: *mut *mut u8,
            ) -> EfiStatus {
                let guid = *guid;
                if guid == GraphicsOutput::GUID {
                    out.write(core::ptr::addr_of!(MOCK_GOP) as *mut _);
                    EfiStatus::SUCCESS
                } else {
                    out.write(null_mut());
                    EfiStatus::NOT_FOUND
                }
            }

            // To update pre-generated CRCs
            #[cfg(no)]
            {
                let mut digest = CRC.digest();
                digest.update(&MOCK_BOOT.0.to_bytes());
                let crc = digest.finalize();
                panic!("crc = {crc:#X}");
            }

            MOCK_BOOT.0.locate_protocol = Some(locate_protocol);

            MOCK_BOOT.0.header.crc32 = {
                let mut digest = CRC.digest();
                digest.update(&MOCK_BOOT.0.to_bytes());
                digest.finalize()
            };

            MOCK_RUN.0.header.crc32 = {
                let mut digest = CRC.digest();
                digest.update(&MOCK_RUN.0.to_bytes());
                digest.finalize()
            };

            s.header.crc32 = {
                let mut digest = CRC.digest();
                digest.update(&s.to_bytes());
                digest.finalize()
            };
        }

        core::ptr::addr_of_mut!(MOCK_SYSTEM)
    }
}

#[repr(transparent)]
struct YesSync<T>(T);
/// Safety: yeah trust me. no
unsafe impl<T> Sync for YesSync<T> {}

// #[derive(Debug)]
#[repr(C)]
pub struct RawBootServices {
    /// Table header
    pub header: Header,

    // Task priority
    pub raise_tpl: *mut u8,
    pub restore_tpl: *mut u8,

    // Memory
    pub allocate_pages: Option<
        unsafe extern "efiapi" fn(
            ty: crate::mem::AllocateType,
            mem_ty: crate::mem::MemoryType,
            pages: usize,
            memory: *mut crate::mem::PhysicalAddress,
        ) -> EfiStatus,
    >,

    pub free_pages: Option<
        unsafe extern "efiapi" fn(
            //
            memory: crate::mem::PhysicalAddress,
            pages: usize,
        ) -> EfiStatus,
    >,

    pub get_memory_map: Option<
        unsafe extern "efiapi" fn(
            map_size: *mut usize,
            map: *mut crate::mem::MemoryDescriptor,
            key: *mut usize,
            entry_size: *mut usize,
            entry_version: *mut u32,
        ) -> EfiStatus,
    >,

    pub allocate_pool: Option<
        unsafe extern "efiapi" fn(
            mem_ty: crate::mem::MemoryType,
            size: usize,
            out: *mut *mut u8,
        ) -> EfiStatus,
    >,

    pub free_pool: Option<unsafe extern "efiapi" fn(mem: *mut u8) -> EfiStatus>,

    // Timers/Events
    pub create_event: *mut u8,
    pub set_timer: *mut u8,
    pub wait_for_event: *mut u8,
    pub signal_event: *mut u8,
    pub close_event: *mut u8,
    pub check_event: *mut u8,

    // Protocols
    pub install_protocol_interface: Option<
        unsafe extern "efiapi" fn(
            handle: *mut EfiHandle,
            guid: *mut proto::Guid,
            interface_ty: u32,
            interface: *mut u8,
        ) -> EfiStatus,
    >,
    pub reinstall_protocol_interface: *mut u8,
    pub uninstall_protocol_interface: *mut u8,
    pub handle_protocol: *mut u8,
    pub reserved: *mut u8,
    pub register_protocol_notify: *mut u8,
    pub locate_handle: *mut u8,
    pub locate_device_path: *mut u8,
    pub install_configuration_table: *mut u8,

    // Images
    pub load_image: Option<
        unsafe extern "efiapi" fn(
            policy: bool,
            parent: EfiHandle,
            path: *mut DevicePath,
            source: *mut u8,
            source_size: usize,
            out: *mut EfiHandle,
        ) -> EfiStatus,
    >,

    pub start_image: Option<
        unsafe extern "efiapi" fn(
            //
            handle: EfiHandle,
            exit_size: *mut usize,
            exit: *mut *mut u8,
        ) -> EfiStatus,
    >,

    pub exit: Option<
        unsafe extern "efiapi" fn(
            handle: EfiHandle,
            status: EfiStatus,
            data_size: usize,
            data: proto::Str16,
        ) -> EfiStatus,
    >,

    pub unload_image: Option<unsafe extern "efiapi" fn(handle: EfiHandle) -> EfiStatus>,

    pub exit_boot_services:
        Option<unsafe extern "efiapi" fn(handle: EfiHandle, key: usize) -> EfiStatus>,

    // Misc
    pub get_next_monotonic_count: Option<unsafe extern "efiapi" fn(count: *mut u64) -> EfiStatus>,

    pub stall: Option<unsafe extern "efiapi" fn(microseconds: usize) -> EfiStatus>,

    pub set_watchdog_timer: Option<
        unsafe extern "efiapi" fn(
            timeout: usize,
            code: u64,
            data_size: usize,
            data: proto::Str16,
        ) -> EfiStatus,
    >,

    // Drivers
    pub connect_controller: *mut u8,
    pub disconnect_controller: *mut u8,

    // Protocols again
    pub open_protocol: Option<
        unsafe extern "efiapi" fn(
            handle: EfiHandle,
            guid: *mut proto::Guid,
            out: *mut *mut u8,
            agent_handle: EfiHandle,
            controller_handle: EfiHandle,
            attributes: u32,
        ) -> EfiStatus,
    >,

    pub close_protocol: Option<
        unsafe extern "efiapi" fn(
            handle: EfiHandle,
            guid: *mut proto::Guid,
            agent_handle: EfiHandle,
            controller_handle: EfiHandle,
        ) -> EfiStatus,
    >,
    pub open_protocol_information: *mut u8,

    // Library?
    pub protocols_per_handle: *mut u8,
    pub locate_handle_buffer: *mut u8,

    pub locate_protocol: Option<
        unsafe extern "efiapi" fn(
            //
            guid: *mut proto::Guid,
            key: *mut u8,
            out: *mut *mut u8,
        ) -> EfiStatus,
    >,

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

    fn to_bytes(&self) -> [u8; size_of::<Self>()] {
        // Safety: `self` is valid by definition
        // Lifetime is bound to self
        unsafe {
            //
            core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>())
                .try_into()
                .unwrap()
        }
    }

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
        t
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

    fn to_bytes(&self) -> [u8; size_of::<Self>()] {
        // Safety: `self` is valid by definition
        // Lifetime is bound to self
        unsafe {
            //
            core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>())
                .try_into()
                .unwrap()
        }
    }

    const fn mock() -> Self {
        const MOCK_HEADER: Header = Header {
            signature: RawRuntimeServices::SIGNATURE,
            revision: Revision::new(2, 70),
            size: size_of::<RawRuntimeServices>() as u32,
            crc32: 0,
            reserved: 0,
        };
        let b = [0u8; size_of::<Self>()];
        // Safety:
        let mut t: RawRuntimeServices = unsafe { core::mem::transmute::<_, _>(b) };
        t.header = MOCK_HEADER;
        t
    }
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
