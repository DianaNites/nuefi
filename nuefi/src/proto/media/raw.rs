use crate::{error::EfiStatus, proto::device_path::raw::RawDevicePath};

pub type LoadFile2Fn = unsafe extern "efiapi" fn(
    this: *mut RawLoadFile2,
    path: *mut RawDevicePath,
    boot: bool,
    buf_size: *mut usize,
    buf: *mut u8,
) -> EfiStatus;

/// UEFI LoadFile2 protocol
#[repr(C)]
pub struct RawLoadFile2 {
    pub load_file: Option<LoadFile2Fn>,
}

impl RawLoadFile2 {
    /// Create a new instance of this protocol
    pub fn create(load_file: LoadFile2Fn) -> Self {
        Self {
            load_file: Some(load_file),
        }
    }
}

pub type OpenVolume = unsafe extern "efiapi" fn(
    //
    this: *mut RawSimpleFileSystem,
    root: *mut *mut RawFile,
) -> EfiStatus;

/// UEFI Simple File System protocol
#[repr(C)]
pub struct RawSimpleFileSystem {
    /// Currently `0x00010000`
    pub revision: u64,
    pub open_volume: Option<OpenVolume>,
}

impl RawSimpleFileSystem {
    //
}

pub type Open = unsafe extern "efiapi" fn(
    //
    this: *mut RawFile,
    new: *mut *mut RawFile,
    name: *const u16,
    mode: u64,
    attributes: u64,
) -> EfiStatus;

pub type Close = unsafe extern "efiapi" fn(this: *mut RawFile) -> EfiStatus;

/// UEFI File protocol
#[repr(C)]
pub struct RawFile {
    /// Currently `0x00020000`
    pub revision: u64,
    pub open: Option<Open>,
    pub close: Option<Close>,
}
