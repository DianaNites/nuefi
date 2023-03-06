//! Raw UEFI data types
use core::{
    mem::{size_of, transmute, ManuallyDrop, MaybeUninit},
    ptr::{null_mut, NonNull},
    slice::from_raw_parts,
};

use crate::{
    error::{EfiStatus, Result},
    proto::{
        self,
        console::raw::{RawSimpleTextInput, RawSimpleTextOutput},
        device_path::raw::RawDevicePath,
        graphics::raw::RawGraphicsOutput,
        Guid,
        Protocol,
    },
    EfiHandle,
};

/// The CRC used by the UEFI tables
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

impl PartialEq<(u32, u32)> for Revision {
    #[inline]
    fn eq(&self, other: &(u32, u32)) -> bool {
        (self.major(), self.minor()).eq(other)
    }
}

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

/// The common header of the 3 UEFI tables, System, Boot, and Runtime.
///
/// This structure precedes the UEFI defined tables. UEFI tables are dynamically
/// sized, but we only need to care about the fields defined here.
///
/// # Safety
///
/// While the header is always the same size, the tables are `size` bytes in
/// memory and it is important this size be used when copying or validating
/// memory.
///
/// As such, these headers and tables, when used from UEFI, *must* only be
/// used via pointers or references.
///
/// It is assumed that memory is valid for all of `size`, and is a single
/// "object" in memory. We trust that this is the case from firmware
/// within [`Header::validate`].
///
/// There is no guarantee the static definitions do not contain
/// padding which must still be initialized from the Rust side if used in, say,
/// mocking. [`RawSystemTable`] is an example of this.
///
/// If you are using this structure manually, be sure to take note of
/// this. You will need to make sure your entire memory allocation is zeroed.
///
/// See <https://github.com/rust-lang/unsafe-code-guidelines/issues/395>
/// for some more details on the rules around padding in Rust.
#[derive(Debug)]
#[repr(C)]
pub struct Header {
    /// Unique signature identifying the table
    pub signature: u64,

    /// The UEFI specification revision which this table claims conformance to
    pub revision: Revision,

    /// Size of the entire table, including this header (24)
    pub size: u32,

    /// 32-bit CRC for the table.
    /// This is set to 0 and computed for `size` bytes.
    ///
    /// See [`CRC`]
    pub crc32: u32,

    /// Reserved field. 0.
    pub reserved: u32,
}

impl Header {
    /// Validate the header for a table with signature `sig`
    ///
    /// This does some basic sanity checks on the UEFI system table,
    /// to ensure this is in fact a proper SystemTable.
    ///
    /// Specifically, it will, in order:
    ///
    /// - Verify that `table` is not null
    /// - Verify the Signature matches `sig`
    /// - Verify that `size` is at least [`size_of::<Header>`] because we're
    ///   paranoid
    /// - Ensure the UEFI major revision is 2, EFI 1.x is not supported, and a
    ///   hypothetical UEFI 3.x is not
    /// - Verify the CRC over `HeaderSize` bytes
    ///
    /// # Safety
    ///
    /// - `table` must be valid for at least [`size_of::<Header>`] bytes
    /// - `table` must contain a valid [`Header`]
    /// - `table` must be valid for [`Header::size`] bytes
    pub unsafe fn validate(table: *const u8, sig: u64) -> Result<()> {
        if table.is_null() {
            return EfiStatus::INVALID_PARAMETER.into();
        }

        // Safety:
        // - `table` is not null
        // - valid UEFI tables contain a `Header`
        // - Callers responsibility
        let header = &*(table as *const Self);
        let len = header.size as usize;

        if header.signature != sig || len < size_of::<Header>() {
            return EfiStatus::INVALID_PARAMETER.into();
        }

        if header.revision.major() != 2 {
            return EfiStatus::INCOMPATIBLE_VERSION.into();
        }

        let expected = header.crc32;

        // Calculate the CRC
        let mut digest = CRC.digest();
        // Native endian because these aren't arrays, we're just viewing them as such
        digest.update(&header.signature.to_ne_bytes());
        digest.update(&header.revision.0.to_ne_bytes());
        digest.update(&header.size.to_ne_bytes());
        digest.update(&0u32.to_ne_bytes());
        digest.update(&header.reserved.to_ne_bytes());

        // Safety:
        // - `table` is subject to caller and earlier validation checks
        // - See [`Header`]
        unsafe {
            let rem = len
                .checked_sub(size_of::<Header>())
                .ok_or(EfiStatus::INVALID_PARAMETER)?;
            // This is always in bounds or 1 past the end because
            // we check the size after the signature
            // `rem` will be valid or the function returned
            let ptr = table.add(size_of::<Header>());
            let bytes = core::slice::from_raw_parts(ptr, rem);

            // Calculate the remaining table, header digested above.
            digest.update(bytes);
        };

        if expected != digest.finalize() {
            return EfiStatus::CRC_ERROR.into();
        }
        Ok(())
    }
}

/// A generic UEFI Configuration table
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawConfigurationTable {
    pub guid: Guid,
    pub table: *mut u8,
}

/// The EFI system table.
///
/// After a call to [`ExitBootServices`], only the following fields are valid:
///
/// - [`RawSystemTable::header`]
/// - [`RawSystemTable::firmware_vendor`]
/// - [`RawSystemTable::firmware_revision`]
/// - [`RawSystemTable::runtime_services`]
/// - [`RawSystemTable::number_of_table_entries`]
/// - [`RawSystemTable::configuration_table`]
///
/// The other fields will be set to null by firmware according to UEFI 7.4.6
// Only valid on x86_64 for now, for safety
#[cfg(target_arch = "x86_64")]
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
    /// See [`Header`]
    ///
    /// FIXME: Figure out what padding is like on 32-bit, if any
    ///
    /// FIXME: Figure out if its actually 100% ABI equivalent to add this
    /// I initially considered removing this and having a utility function that
    /// returns `Self` in a `Box`, but then I remembered: COMPOSING!
    /// This wouldn't compose well at all.
    ///
    /// The safety of this is justified under assuming C zeros it
    /// and this is ABI equiv, or we created it and zeroed it.
    pub _pad1: [u8; 4],

    /// Console input handle
    pub console_in_handle: EfiHandle,

    /// Console input protocol
    pub con_in: *mut RawSimpleTextInput,

    /// Console output handle
    pub console_out_handle: EfiHandle,

    /// Console output protocol
    pub con_out: *mut RawSimpleTextOutput,

    /// Console error handle
    pub console_err_handle: EfiHandle,

    /// Console error output
    pub con_err: *mut RawSimpleTextOutput,

    /// Runtime services table, always valid
    pub runtime_services: *mut RawRuntimeServices,

    /// Boot services table
    pub boot_services: *mut RawBootServices,

    /// Number of entries in `configuration_table`
    pub number_of_table_entries: usize,

    /// Configuration table, always valid
    pub configuration_table: *mut RawConfigurationTable,
}

impl RawSystemTable {
    pub const SIGNATURE: u64 = 0x5453595320494249;

    /// Validate the table
    ///
    ///
    /// # Safety
    ///
    /// - `this` must be valid for [`size_of::<RawSystemTable>`] bytes
    /// - `this` must contain a valid [`RawSystemTable`]
    ///
    /// See [`Header::validate`] for details
    pub unsafe fn validate(this: *mut Self) -> Result<()> {
        // Safety: Validating ourself, callers responsibility
        Header::validate(this as *const u8, Self::SIGNATURE)?;

        let header = &(*this);

        // Safety: Callers responsibility
        Header::validate(
            header.boot_services as *const u8,
            RawBootServices::SIGNATURE,
        )?;
        Header::validate(
            header.runtime_services as *const u8,
            RawRuntimeServices::SIGNATURE,
        )?;

        Ok(())
    }
}

/// Search type for
/// [`RawBootServices::locate_handle`] and
/// [`RawBootServices::locate_handle_buffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocateSearch(u32);

impl LocateSearch {
    /// Protocol and SearchKey are ignored, every handle in the system is
    /// returned.
    pub const ALL_HANDLES: Self = Self(0);

    /// SearchKey supplies a Registration value from
    /// [`RawBootServices::register_protocol_notify`].
    ///
    /// The next handle that is new for registration is returned.
    /// Only one handle is returned at a time.
    pub const BY_REGISTER_NOTIFY: Self = Self(1);

    /// All handles that support protocol are returned
    pub const BY_PROTOCOL: Self = Self(2);
}

/// Locate handles, determined by the parameters
pub type LocateHandle = unsafe extern "efiapi" fn(
    search_type: LocateSearch,
    protocol: *const Guid,
    search_key: *const u8,
    buffer_size: *mut usize,
    buffer: *mut EfiHandle,
) -> EfiStatus;

pub type HandleProtocolFn = unsafe extern "efiapi" fn(
    handle: EfiHandle,
    guid: *const Guid,
    interface: *mut *mut u8,
) -> EfiStatus;

pub type LocateProtocolFn = unsafe extern "efiapi" fn(
    //
    guid: *mut proto::Guid,
    key: *mut u8,
    out: *mut *mut u8,
) -> EfiStatus;

pub type InstallConfigurationTable = unsafe extern "efiapi" fn(
    //
    guid: *mut proto::Guid,
    table: *mut u8,
) -> EfiStatus;

/// Raw structure of the UEFI Boot Services table
/// NOTE: It is important for safety that all fields be nullable.
/// In particular, this means fn pointers MUST be wrapped in [`Option`].
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
    pub handle_protocol: Option<HandleProtocolFn>,
    pub _reserved: *mut u8,
    pub register_protocol_notify: *mut u8,

    pub locate_handle: Option<LocateHandle>,

    pub locate_device_path: *mut u8,
    pub install_configuration_table: Option<InstallConfigurationTable>,

    // Images
    pub load_image: Option<
        unsafe extern "efiapi" fn(
            policy: bool,
            parent: EfiHandle,
            path: *mut RawDevicePath,
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

    pub locate_protocol: Option<LocateProtocolFn>,

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
    pub const SIGNATURE: u64 = 0x56524553544f4f42;
}

#[derive(Debug)]
#[repr(C)]
pub struct RawRuntimeServices {
    /// Table header
    pub header: Header,
}

impl RawRuntimeServices {
    pub const SIGNATURE: u64 = 0x56524553544e5552;
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
