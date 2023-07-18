//! Definitions for the UEFI System tables
//!
//! This provides fully public FFI-compatible definitions for the UEFI tables.
//!
//! It also attempts to provide safer ways to construct known valid variants
use core::{ffi::c_void, mem::size_of};

use crate::{base::*, error::Result};

pub mod boot_fn;
pub mod config;
pub mod mem;

// FIXME: Hack
type SimpleTextInput = c_void;
// FIXME: Hack
type SimpleTextOutput = c_void;

/// The CRC used by the UEFI tables
///
/// See [`Header`]
// FIXME: UEFI doesn't actually require this, and provides CalculateCrc32 for
// this. The problem, however, is to use it you must first blindly trust the
// system table.
// Maybe try it only as a fallback, just in case? who knows what firmware does
pub static CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

/// UEFI Header Revision
///
/// This is a binary coded decimal.
///
/// The upper 16 bits are the major version
///
/// The lower 16 bits are the minor version, in binary coded decimal
///
/// Same representation as [`u32`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Revision(pub u32);

impl Revision {
    /// Create a new revision for `major.minor`
    #[inline]
    pub const fn new(major: u16, minor: u16) -> Self {
        Revision(((major as u32) << 16) | minor as u32)
    }

    /// The major part of the revision
    ///
    /// X in `X.y.z`
    #[inline]
    pub const fn major(self) -> u32 {
        self.0 >> 16
    }

    /// The minor part of the revision
    ///
    /// Limited to 0-9
    ///
    /// Y in `x.Y.z`
    #[inline]
    pub const fn minor(self) -> u32 {
        self.0 as u16 as u32 / 10
    }

    /// The patch part of the revision
    ///
    /// Limited to 0-9
    ///
    /// Z in `x.y.Z`
    #[inline]
    pub const fn patch(self) -> u32 {
        self.0 as u16 as u32 % 10
    }
}

impl core::fmt::Display for Revision {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.major(), self.minor())?;
        if self.patch() > 0 {
            write!(f, ".{}", self.patch())?;
        }
        Ok(())
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
/// mocking. [`SystemTable`] is an example of this.
///
/// If you are using this structure manually, be sure to take note of
/// this. You will need to make sure your entire memory allocation is zeroed.
///
/// See <https://github.com/rust-lang/unsafe-code-guidelines/issues/395>
/// for some more details on the rules around padding in Rust.
///
/// Also see <https://users.rust-lang.org/t/is-it-possible-to-read-uninitialized-memory-without-invoking-ub/63092/17>
#[derive(Debug, Clone, Copy)]
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
    /// - Verify that [`Header::signature`] matches `sig`
    /// - Verify that [`Header::size`] is at least as expected by `sig` because
    ///   we're paranoid
    /// - Verify [`Header::revision`] is `2.x`. EFI `1.x` is not supported, and
    ///   a hypothetical UEFI `3.x` is not
    /// - Verify [`Header::crc32`] over [`Header::size`] bytes
    ///
    /// # Safety
    ///
    /// - If not null, `table` must:
    ///   - Be valid for at least [`size_of::<Header>`] bytes
    ///   - Contain a valid [`Header`]
    ///   - Be valid for [`Header::size`] bytes
    ///   - Contain a valid table as determined by `sig`
    pub unsafe fn validate(table: *const u8, sig: u64) -> Result<()> {
        if table.is_null() {
            return Status::INVALID_PARAMETER.into();
        }

        // Safety:
        // - `table` is not null
        // - valid UEFI tables contain a `Header`
        // - Callers responsibility
        let header = &*(table as *const Self);
        let len = header.size as usize;

        if header.signature != sig {
            return Status::INVALID_PARAMETER.into();
        }

        let expected_size = if sig == SystemTable::SIGNATURE {
            size_of::<SystemTable>()
        } else if sig == RuntimeServices::SIGNATURE {
            size_of::<RuntimeServices>()
        } else if sig == BootServices::SIGNATURE {
            size_of::<BootServices>()
        } else {
            return Status::INVALID_PARAMETER.into();
        };

        // Make sure size is enough
        if len < expected_size {
            return Status::INVALID_PARAMETER.into();
        }

        if header.revision.major() != 2 {
            return Status::INCOMPATIBLE_VERSION.into();
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
                .ok_or(Status::INVALID_PARAMETER)?;
            // This is always in bounds or 1 past the end because
            // we check the size after the signature
            // `rem` will be valid or the function returned
            let ptr = table.add(size_of::<Header>());
            let bytes = core::slice::from_raw_parts(ptr, rem);

            // Calculate the remaining table, header digested above.
            digest.update(bytes);
        };

        if expected != digest.finalize() {
            return Status::CRC_ERROR.into();
        }
        Ok(())
    }
}

/// The EFI system table.
///
/// After a call to [`ExitBootServices`], only the following fields are valid:
///
/// - [`SystemTable::header`]
/// - [`SystemTable::firmware_vendor`]
/// - [`SystemTable::firmware_revision`]
/// - [`SystemTable::runtime_services`]
/// - [`SystemTable::number_of_table_entries`]
/// - [`SystemTable::configuration_table`]
///
/// The other fields will be set to null by firmware according to UEFI Section
/// 7.4.6
///
/// This is FFI-safe
// Only valid on x86_64 for now, for safety
#[cfg(target_arch = "x86_64")]
#[derive(Debug)]
#[repr(C)]
pub struct SystemTable {
    /// Table header, always valid
    pub header: Header,

    /// Firmware vendor, always valid
    ///
    /// Null terminated UCS-2 string
    pub firmware_vendor: *mut Char16,

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
    pub console_in_handle: Handle,

    /// Console input protocol
    pub con_in: *mut SimpleTextInput,

    /// Console output handle
    pub console_out_handle: Handle,

    /// Console output protocol
    pub con_out: *mut SimpleTextOutput,

    /// Console error handle
    pub console_err_handle: Handle,

    /// Console error output
    pub con_err: *mut SimpleTextOutput,

    /// Runtime services table, always valid
    pub runtime_services: *mut RuntimeServices,

    /// Boot services table
    pub boot_services: *mut BootServices,

    /// Number of entries in `configuration_table`
    pub number_of_table_entries: usize,

    /// Configuration table, always valid
    pub configuration_table: *mut config::ConfigurationTable,
}

impl SystemTable {
    pub const SIGNATURE: u64 = 0x5453595320494249;

    pub const REVISION: Revision = Self::REVISION_2_100;
    pub const REVISION_2_100: Revision = Revision::new(2, 100);
    pub const REVISION_2_90: Revision = Revision::new(2, 90);
    pub const REVISION_2_80: Revision = Revision::new(2, 80);
    pub const REVISION_2_70: Revision = Revision::new(2, 70);
    pub const REVISION_2_60: Revision = Revision::new(2, 60);
    pub const REVISION_2_50: Revision = Revision::new(2, 50);
    pub const REVISION_2_40: Revision = Revision::new(2, 40);
    pub const REVISION_2_31: Revision = Revision::new(2, 31);
    pub const REVISION_2_30: Revision = Revision::new(2, 30);
    pub const REVISION_2_20: Revision = Revision::new(2, 20);
    pub const REVISION_2_00: Revision = Revision::new(2, 00);
    pub const REVISION_1_10: Revision = Revision::new(1, 10);
    pub const REVISION_1_02: Revision = Revision::new(1, 2);

    pub const SPECIFICATION: Revision = Self::REVISION;

    /// Validate the table
    ///
    ///
    /// # Safety
    ///
    /// - `this` must be valid for [`size_of::<SystemTable>`] bytes
    /// - `this` must contain a valid [`SystemTable`]
    ///
    /// See [`Header::validate`] for details
    pub unsafe fn validate(this: *mut Self) -> Result<()> {
        // Safety: Validating ourself, callers responsibility
        Header::validate(this as *const u8, Self::SIGNATURE)?;

        let header = &(*this);

        // Safety: Callers responsibility
        Header::validate(header.boot_services as *const u8, BootServices::SIGNATURE)?;
        Header::validate(
            header.runtime_services as *const u8,
            RuntimeServices::SIGNATURE,
        )?;

        Ok(())
    }
}

/// Search type for
/// [`BootServices::locate_handle`] and
/// [`BootServices::locate_handle_buffer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocateSearch(u32);

impl LocateSearch {
    /// Protocol and SearchKey are ignored, every handle in the system is
    /// returned.
    pub const ALL_HANDLES: Self = Self(0);

    /// SearchKey supplies a Registration value from
    /// [`BootServices::register_protocol_notify`].
    ///
    /// The next handle that is new for registration is returned.
    /// Only one handle is returned at a time.
    pub const BY_REGISTER_NOTIFY: Self = Self(1);

    /// All handles that support protocol are returned
    pub const BY_PROTOCOL: Self = Self(2);
}

/// The UEFI Boot Services Table
///
/// This is FFI-safe
#[repr(C)]
pub struct BootServices {
    /// Table header
    pub header: Header,

    // Task priority
    pub raise_tpl: *mut c_void,
    pub restore_tpl: *mut c_void,

    // Memory
    pub allocate_pages: Option<boot_fn::AllocatePages>,

    pub free_pages: Option<boot_fn::FreePages>,

    pub get_memory_map: Option<boot_fn::GetMemoryMap>,

    pub allocate_pool: Option<boot_fn::AllocatePool>,

    pub free_pool: Option<boot_fn::FreePool>,

    // Timers/Events
    pub create_event: *mut c_void,
    pub set_timer: *mut c_void,
    pub wait_for_event: *mut c_void,
    pub signal_event: *mut c_void,
    pub close_event: *mut c_void,
    pub check_event: *mut c_void,

    // Protocols
    pub install_protocol_interface: Option<boot_fn::InstallProtocolInterface>,
    pub reinstall_protocol_interface: *mut c_void,
    pub uninstall_protocol_interface: *mut c_void,
    pub handle_protocol: Option<boot_fn::HandleProtocolFn>,
    pub _reserved: *mut c_void,
    pub register_protocol_notify: *mut c_void,

    pub locate_handle: Option<boot_fn::LocateHandle>,

    pub locate_device_path: *mut c_void,
    pub install_configuration_table: Option<boot_fn::InstallConfigurationTable>,

    // Images
    pub load_image: Option<boot_fn::LoadImage>,

    pub start_image: Option<boot_fn::StartImage>,

    pub exit: Option<boot_fn::Exit>,

    pub unload_image: Option<boot_fn::UnloadImage>,

    pub exit_boot_services: Option<boot_fn::ExitBootServices>,

    // Misc
    pub get_next_monotonic_count: Option<boot_fn::GetNextMonotonicCount>,

    pub stall: Option<boot_fn::Stall>,

    pub set_watchdog_timer: Option<boot_fn::SetWatchdogTimer>,

    // Drivers
    pub connect_controller: *mut c_void,
    pub disconnect_controller: *mut c_void,

    // Protocols again
    pub open_protocol: Option<boot_fn::OpenProtocol>,

    pub close_protocol: Option<boot_fn::CloseProtocol>,
    pub open_protocol_information: *mut c_void,

    // Library?
    pub protocols_per_handle: *mut c_void,
    pub locate_handle_buffer: *mut c_void,

    pub locate_protocol: Option<boot_fn::LocateProtocolFn>,

    pub install_multiple_protocol_interfaces: *mut c_void,
    pub uninstall_multiple_protocol_interfaces: *mut c_void,

    // Useless CRC
    pub calculate_crc32: *mut c_void,

    // Misc again
    pub copy_mem: *mut c_void,
    pub set_mem: *mut c_void,
    pub create_event_ex: *mut c_void,
}

impl BootServices {
    pub const SIGNATURE: u64 = 0x56524553544f4f42;
    pub const REVISION: Revision = SystemTable::SPECIFICATION;
}

/// The UEFI Runtime Services Table
#[derive(Debug)]
#[repr(C)]
pub struct RuntimeServices {
    /// Table header
    pub header: Header,
}

impl RuntimeServices {
    pub const SIGNATURE: u64 = 0x56524553544e5552;
    pub const REVISION: Revision = SystemTable::SPECIFICATION;
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
