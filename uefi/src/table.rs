//! UEFI Tables

use core::mem::size_of;

use crate::{
    error::{EfiStatus, Result},
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
pub struct SystemTable {
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
    con_in: Void, // EFI_SIMPLE_TEXT_INPUT_PROTOCOL

    ///
    console_out_handle: EfiHandle,

    ///
    con_out: Void, // EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL

    ///
    standard_error_handle: EfiHandle,

    ///
    std_err: Void, // EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL

    /// Runtime services table, always valid
    runtime_services: *mut RuntimeServices,

    /// Boot services table
    boot_services: *mut BootServices,

    /// Number of entries, always valid
    number_of_table_entries: usize,

    /// Configuration table, always valid
    configuration_table: Void, // EFI_CONFIGURATION_TABLE
}

impl SystemTable {
    const SIGNATURE: u64 = 0x5453595320494249;

    /// Validate the table
    ///
    /// Validation fails if CRC validation fails, or the UEFI revision is
    /// unsupported
    ///
    /// # Safety
    ///
    /// - Must be a valid pointer
    /// - Must only e called before running user code.
    pub(crate) unsafe fn validate(this: *mut Self) -> Result<()> {
        // Safety: Pointer to first C struct member
        Header::validate(this as *mut Header, Self::SIGNATURE)?;
        let header = &(*this);
        Header::validate(header.boot_services as *mut Header, BootServices::SIGNATURE)?;
        Header::validate(
            header.runtime_services as *mut Header,
            RuntimeServices::SIGNATURE,
        )?;
        Ok(())
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BootServices {
    /// Table header
    header: Header,
}

impl BootServices {
    const SIGNATURE: u64 = 0x56524553544f4f42;
}

#[derive(Debug)]
#[repr(C)]
pub struct RuntimeServices {
    /// Table header
    header: Header,
}

impl RuntimeServices {
    const SIGNATURE: u64 = 0x56524553544e5552;
}
