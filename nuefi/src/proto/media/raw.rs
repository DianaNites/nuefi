use crate::{
    error::EfiStatus,
    proto::{device_path::raw::RawDevicePath, Guid, Time},
};

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
    this: *mut RawFile,
    new: *mut *mut RawFile,
    name: *const u16,
    mode: u64,
    attributes: u64,
) -> EfiStatus;

pub type GetInfo = unsafe extern "efiapi" fn(
    this: *mut RawFile,
    info_type: *const Guid,
    buffer_size: *mut usize,
    buffer: *mut u8,
) -> EfiStatus;

pub type Close = unsafe extern "efiapi" fn(this: *mut RawFile) -> EfiStatus;

pub type Read = unsafe extern "efiapi" fn(
    this: *mut RawFile,
    buffer_size: *mut usize,
    buffer: *mut u8,
) -> EfiStatus;

/// UEFI File protocol
#[repr(C)]
pub struct RawFile {
    /// Currently `0x00020000`
    pub revision: u64,

    pub open: Option<Open>,
    pub close: Option<Close>,
    pub delete: *const u8,
    pub read: Option<Read>,
    pub write: *const u8,

    pub get_pos: *const u8,
    pub set_pos: *const u8,

    pub get_info: Option<GetInfo>,

    pub set_info: *const u8,
    pub flush: *const u8,

    // Below added in revision 2
    pub open_ex: *const u8,
    pub read_ex: *const u8,
    pub write_ex: *const u8,
    pub flush_ex: *const u8,
}

/// UEFI [`RawFile`] information
#[derive(Debug)]
#[repr(C)]
pub struct RawFileInfo {
    pub size: u64,
    pub file_size: u64,
    pub physical_size: u64,
    pub create_time: Time,
    pub last_access_time: Time,
    pub modification_time: Time,
    pub flags: u64,
    // This type is dynamically sized
    // pub filename: *mut u16,
}
