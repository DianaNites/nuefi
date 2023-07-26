//! UEFI Loaded image Protocol
use core::{mem::size_of, slice::from_raw_parts};

use nuefi_core::{
    error::{Result, Status},
    interface,
    proto::device_path::DevicePathHdr,
};
use raw::RawLoadedImage;

use super::{device_path::DevicePath, Guid, Protocol};
use crate::{
    string::{Path, UefiStr},
    EfiHandle,
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("5B1B31A1-9562-11D2-8E3F-00A0C969723B")]
    LoadedImage(RawLoadedImage)
);

impl<'table> LoadedImage<'table> {
    const _REVISION: u32 = 0x1000;

    /// The [`Path`] to the file of the loaded image, if it exists.
    ///
    /// Note that this does not include the *device* the file is on,
    /// and so does not identify where this image was loaded from.
    /// For that see [`LoadedImageDevicePath`]
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
        if !self.interface().device.as_ptr().is_null() {
            Some(self.interface().device)
        } else {
            None
        }
    }

    /// Read the options for this image as a [`&[u8]`]
    pub fn options(&self) -> Option<Result<&[u8]>> {
        let i = self.interface();
        let opts = i.options;
        if opts.is_null() || i.options_size == 0 {
            None
        } else {
            let len = i.options_size as usize;
            // Safety: opts is valid
            unsafe { Some(Ok(from_raw_parts(opts, len))) }
        }
    }

    /// Read the options for this image as a [`UefiStr`], if they exist and are
    /// valid.
    pub fn shell_options(&self) -> Option<Result<UefiStr>> {
        let i = self.interface();
        let opts = i.options;
        if opts.is_null() || i.options_size == 0 {
            None
        } else {
            let opts = opts as *mut u16;

            let len = i.options_size as usize / 2;
            if i.options_size % 2 != 0 {
                return Some(Err(Status::INVALID_PARAMETER.into()));
            }
            // Safety: Unsafe
            Some(Ok(unsafe { UefiStr::from_ptr_len(opts, len) }))
        }
    }

    /// Set the LoadOptions for this loaded image
    ///
    /// # Panics
    ///
    /// - If `data` is bigger than [`u32::MAX`], in bytes.
    ///
    /// # Safety
    ///
    /// - `data` MUST live until [`BootServices::start_image`][start_image] is
    ///   called
    ///
    /// [start_image]: crate::table::BootServices::start_image
    pub unsafe fn set_options<T>(&self, data: &[T]) {
        // Safety: Existence of `&self` implies validity
        let i = unsafe { &mut *self.interface };

        let len: u32 = data.len().try_into().unwrap();
        let size: u32 = size_of::<T>().try_into().unwrap();

        i.options = data.as_ptr() as *mut u8;
        i.options_size = len * size;
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
        // Safety: Always correct for shell options
        self.set_options::<u16>(cmd.as_slice_with_nul());
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
        // Safety: Existence of `&self` implies validity
        unsafe { &mut *self.interface }.device = device;
    }

    /// Set the [DevicePath] for this image
    ///
    /// # Safety
    ///
    /// Only use this if you know what you're doing
    pub unsafe fn set_path(&self, path: &Path<'_>) {
        // Safety: Existence of `&self` implies validity
        unsafe { &mut *self.interface }.path = path.as_device().as_ptr();
    }
}

interface!(
    /// Identical to [`DevicePath`] except for the GUID
    #[Protocol("BC62157E-3E33-4FEC-9920-2D3B36D750DF")]
    LoadedImageDevicePath(DevicePathHdr)
);

impl<'table> LoadedImageDevicePath<'table> {
    pub fn as_device_path(&self) -> DevicePath<'_> {
        // Safety: This is a DevicePath
        unsafe { DevicePath::from_raw(self.interface) }
    }
}
