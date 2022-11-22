//! UEFI Device Path Protocol
use alloc::{string::String, vec::Vec};

use log::{error, info, trace};

use super::{Guid, Protocol, Str16};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    string::{string_len, UefiString},
    util::interface,
};

/// Raw device path structure
///
/// Device Paths are variable length, unaligned/byte packed structures.
///
/// All fields must be assumed unaligned
///
/// Also a protocol that can be used on any handle to obtain its path, if it
/// exists.
#[derive(Debug)]
#[repr(C, packed)]
pub(crate) struct RawDevicePath {
    ty: u8,
    sub_ty: u8,
    /// Length, including this header
    len: [u8; 2],
}

interface!(DevicePath(RawDevicePath));

impl<'table> DevicePath<'table> {
    //
}

unsafe impl<'table> Protocol<'table> for DevicePath<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x09, 0x57, 0x6e, 0x91, 0x6d, 0x3f, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69,
            0x72, 0x3b,
        ])
    };

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { DevicePath::new(this as *mut RawDevicePath) }
    }
}

/// Device Path Utilities protocol
// #[derive(Debug)]
#[repr(C)]
pub(crate) struct RawDevicePathUtil {
    get_device_path_size: unsafe extern "efiapi" fn(this: *mut RawDevicePath) -> usize,
    duplicate_device_path: *mut u8,
    append_device_path: *mut u8,
    append_device_node: *mut u8,
    append_device_path_instance: *mut u8,
    get_next_device_path_instance: *mut u8,
    is_device_path_multi_instance: *mut u8,
    create_device_node: *mut u8,
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

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { DevicePathUtil::new(this as *mut RawDevicePathUtil) }
    }
}

/// Device Path Display protocol
// #[derive(Debug)]
#[repr(C)]
pub(crate) struct RawDevicePathToText {
    convert_device_node_to_text: unsafe extern "efiapi" fn(
        node: *mut RawDevicePath,
        display: bool,
        shortcuts: bool,
    ) -> *mut u16,
    convert_device_path_to_text: unsafe extern "efiapi" fn(
        path: *mut RawDevicePath,
        display: bool,
        shortcuts: bool,
    ) -> *mut u16,
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
            Ok(unsafe { UefiString::from_raw(ret, string_len(ret)) })
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
            Ok(unsafe { UefiString::from_raw(ret, string_len(ret)) })
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

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { DevicePathToText::new(this as *mut RawDevicePathToText) }
    }
}
