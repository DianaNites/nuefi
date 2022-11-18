//! UEFI Tables

use core::{marker::PhantomData, mem::size_of};

use crate::{
    error::{EfiStatus, Result},
    proto::{self, RawSimpleTextInput, RawSimpleTextOutput, SimpleTextOutput},
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
    /// - Must only e called before running user code.
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

#[derive(Debug)]
#[repr(C)]
pub struct RawBootServices {
    /// Table header
    header: Header,
}

impl RawBootServices {
    const SIGNATURE: u64 = 0x56524553544f4f42;
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
    /// The UEFI Boot services
    BootServices(RawBootServices),
    ///
    RuntimeServices(RawRuntimeServices),
);

impl<'table> BootServices<'table> {
    //
}

/// Type marker for [`SystemTable`] representing before ExitBootServices is
/// called
pub struct Boot;

/// Type marker for [`SystemTable`] representing after ExitBootServices is
/// called
pub struct Runtime;

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

impl SystemTable<Boot> {
    /// Create new SystemTable
    ///
    /// # Safety
    ///
    /// - Must be valid pointer
    pub(crate) unsafe fn new(this: *mut RawSystemTable) -> Self {
        Self {
            table: this,
            phantom: PhantomData,
        }
    }

    fn table(&self) -> &RawSystemTable {
        unsafe { &*self.table }
    }

    /// String identifying the vendor
    pub fn firmware_vendor(&self) -> &str {
        ""
    }

    /// Firmware-specific value indicating its revision
    pub fn firmware_revision(&self) -> u32 {
        self.table().firmware_revision
    }

    /// Output on stdout
    pub fn stdout(&self) -> SimpleTextOutput<'_> {
        unsafe { SimpleTextOutput::new(self.table().con_out) }
    }

    /// Output on stderr
    pub fn stderr(&self) -> SimpleTextOutput<'_> {
        unsafe { SimpleTextOutput::new(self.table().std_err) }
    }

    pub fn boot(&self) -> Option<BootServices<'_>> {
        if !self.table().boot_services.is_null() {
            Some(unsafe { BootServices::new(self.table().boot_services) })
        } else {
            None
        }
    }
}
