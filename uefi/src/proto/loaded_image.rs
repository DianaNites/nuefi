//! UEFI Loaded image Protocol
use core::mem::size_of;

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

/// Raw UEFI LoadedImage protocol structure
#[derive(Debug)]
#[repr(C)]
pub struct RawLoadedImage {
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

    /// Returns the base address of our executable in memory
    pub fn image_base(&self) -> *mut u8 {
        self.interface().image_base
    }

    /// Returns the size of our executable in memory
    pub fn image_size(&self) -> u64 {
        self.interface().image_size
    }

    /// The device handle that the EFI Image was loaded from, or [None]
    pub fn device(&self) -> Option<EfiHandle> {
        if !self.interface().device.0.is_null() {
            Some(self.interface().device)
        } else {
            None
        }
    }

    /// Set the LoadOptions for this loaded image
    ///
    /// # Panics
    ///
    /// - If `data` is bigger than [`u32::MAX`]
    ///
    /// # Safety
    ///
    /// You should only use this if you know what you're doing.
    ///
    /// It is your responsibility to ensure the data lives long enough until
    /// start_image is called.
    pub unsafe fn set_options<T>(&self, data: &[T]) {
        // EFI pls dont write to our options
        self.interface_mut().options = data.as_ptr() as *mut _;
        let len: u32 = data.len().try_into().unwrap();
        let size: u32 = size_of::<T>().try_into().unwrap();
        self.interface_mut().options_size = len * size;
    }

    /// Set the Device handle for this image
    ///
    /// # Safety
    ///
    /// Only use this if you know what you're doing
    pub unsafe fn set_device(&self, device: EfiHandle) {
        self.interface_mut().device = device;
    }

    /// Set the [DevicePath] for this image
    ///
    /// # Safety
    ///
    /// Only use this if you know what you're doing
    pub unsafe fn set_path(&self, path: &Path) {
        self.interface_mut().path = path.as_device().as_ptr();
    }
}

unsafe impl<'table> Protocol<'table> for LoadedImage<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x5B, 0x1B, 0x31, 0xA1, 0x95, 0x62, 0x11, 0xd2, 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69,
            0x72, 0x3B,
        ])
    };

    type Raw = RawLoadedImage;

    unsafe fn from_raw(this: *mut RawLoadedImage) -> Self {
        unsafe { LoadedImage::new(this) }
    }
}
