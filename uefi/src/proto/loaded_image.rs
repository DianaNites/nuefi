//! UEFI Loaded image Protocol
use super::{
    device_path::{DevicePath, RawDevicePath},
    Guid,
    Protocol,
    Str16,
};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    mem::MemoryType,
    string::Path,
    table::RawSystemTable,
    util::interface,
    EfiHandle,
};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct RawLoadedImage {
    revision: u32,
    parent: EfiHandle,
    system_table: *mut RawSystemTable,

    device: EfiHandle,
    path: *mut RawDevicePath,
    _reserved: *mut u8,

    options_size: u32,
    options: *mut u8,

    image_base: *mut u8,
    image_size: u64,
    image_code: MemoryType,
    image_data: MemoryType,
    unload: *mut u8,
}

interface!(LoadedImage(RawLoadedImage));

impl<'table> LoadedImage<'table> {
    const REVISION: u32 = 0x1000;

    /// The [Path] to the file of the loaded image, if it exists.
    pub fn file_path(&self) -> Option<Path> {
        let path = self.interface().path;
        if !path.is_null() {
            Some(Path::new(unsafe { DevicePath::new(path) }))
        } else {
            None
        }
    }
}

unsafe impl<'table> Protocol<'table> for LoadedImage<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x5B, 0x1B, 0x31, 0xA1, 0x95, 0x62, 0x11, 0xd2, 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69,
            0x72, 0x3B,
        ])
    };

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { LoadedImage::new(this as *mut RawLoadedImage) }
    }
}
