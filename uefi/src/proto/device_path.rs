//! UEFI Device Path Protocol

use super::{Guid, Protocol};
use crate::{
    error::{EfiStatus, Result, UefiError},
    string::UefiString,
    table::BootServices,
    util::interface,
};

pub mod raw;
use raw::{RawDevicePath, RawDevicePathToText, RawDevicePathUtil};

interface!(DevicePath(RawDevicePath));

impl<'table> DevicePath<'table> {
    /// Free the DevicePath
    pub(crate) fn free(&mut self, boot: &BootServices) -> Result<()> {
        unsafe { boot.free_pool(self.interface as *mut u8) }
    }
}

unsafe impl<'table> Protocol<'table> for DevicePath<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x09, 0x57, 0x6e, 0x91, 0x6d, 0x3f, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69,
            0x72, 0x3b,
        ])
    };

    type Raw = RawDevicePath;

    unsafe fn from_raw(this: *mut RawDevicePath) -> Self {
        unsafe { DevicePath::new(this) }
    }
}

interface!(DevicePathUtil(RawDevicePathUtil));

impl<'table> DevicePathUtil<'table> {
    /// [DevicePath] size, in bytes. NOT including the End Of Path node.
    pub fn get_device_path_size(&self, node: &DevicePath) -> usize {
        unsafe {
            (self.interface().get_device_path_size)(node.interface)
                // End of path node
                - core::mem::size_of::<RawDevicePath>()
        }
    }
}

unsafe impl<'table> Protocol<'table> for DevicePathUtil<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x03, 0x79, 0xBE, 0x4E, 0xD7, 0x06, 0x43, 0x7d, 0xB0, 0x37, 0xED, 0xB8, 0x2F, 0xB7,
            0x72, 0xA4,
        ])
    };

    type Raw = RawDevicePathUtil;

    unsafe fn from_raw(this: *mut RawDevicePathUtil) -> Self {
        unsafe { DevicePathUtil::new(this) }
    }
}

interface!(DevicePathToText(RawDevicePathToText));

impl<'table> DevicePathToText<'table> {
    /// Returns an owned [UefiString] of `node`, a component of a [DevicePath]
    ///
    /// # Errors
    ///
    /// - If memory allocation fails
    pub fn convert_device_node_to_text(&self, node: &DevicePath) -> Result<UefiString> {
        let ret =
            unsafe { (self.interface().convert_device_node_to_text)(node.interface, false, false) };
        if !ret.is_null() {
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(UefiError::new(EfiStatus::OUT_OF_RESOURCES))
        }
    }

    /// Returns an owned [UefiString] of `path`
    ///
    /// # Errors
    ///
    /// - If memory allocation fails
    pub fn convert_device_path_to_text(&self, path: &DevicePath) -> Result<UefiString> {
        let ret =
            unsafe { (self.interface().convert_device_path_to_text)(path.interface, false, false) };
        if !ret.is_null() {
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(UefiError::new(EfiStatus::OUT_OF_RESOURCES))
        }
    }
}

unsafe impl<'table> Protocol<'table> for DevicePathToText<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x8b, 0x84, 0x3e, 0x20, 0x81, 0x32, 0x48, 0x52, 0x90, 0xcc, 0x55, 0x1a, 0x4e, 0x4a,
            0x7f, 0x1c,
        ])
    };

    type Raw = RawDevicePathToText;

    unsafe fn from_raw(this: *mut RawDevicePathToText) -> Self {
        unsafe { DevicePathToText::new(this) }
    }
}
