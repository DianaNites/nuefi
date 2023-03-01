//! UEFI Loaded image Protocol
use core::mem::size_of;

use raw::RawLoadedImage;

use super::{device_path::DevicePath, Guid, Protocol};
use crate::{
    string::{Path, UefiStr},
    util::interface,
    EfiHandle,
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("5B1B31A1-9562-11D2-8E3F-00A0C969723B", crate("crate"))]
    LoadedImage(RawLoadedImage)
);

impl<'table> LoadedImage<'table> {
    const _REVISION: u32 = 0x1000;

    /// The [Path] to the file of the loaded image, if it exists.
    pub fn file_path(&self) -> Option<Path<'_>> {
        let path = self.interface().path;
        if !path.is_null() {
            // Safety: `path` is valid
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

    /// Set the image load options in Shell format,
    /// as a UTF-16 null terminated string.
    ///
    /// # Panics
    ///
    /// - If `cmd` is bigger than [`u32::MAX`]
    ///
    /// # Safety
    ///
    /// - You must ensure this image is, in fact,
    /// expecting arguments in this format.
    /// - `cmd` MUST live until [`BootServices::start_image`][start_image] is
    ///   called
    ///
    /// [start_image]: crate::table::BootServices::start_image
    pub unsafe fn set_shell_options(&self, cmd: &UefiStr<'_>) {
        // Safety: Always correct fot shell options
        self.set_options::<u16>(cmd.as_slice());
    }

    /// Set the Device handle for this image
    ///
    /// # Safety
    ///
    /// - `device` must be the [`EfiHandle`] for the device this image was
    ///   loaded from.
    ///
    /// This is normally/should be set [load_image][load_image] when you call
    /// it.
    ///
    /// [load_image]: crate::table::BootServices
    pub unsafe fn set_device(&self, device: EfiHandle) {
        self.interface_mut().device = device;
    }

    /// Set the [DevicePath] for this image
    ///
    /// # Safety
    ///
    /// Only use this if you know what you're doing
    pub unsafe fn set_path(&self, path: &Path<'_>) {
        self.interface_mut().path = path.as_device().as_ptr();
    }
}
