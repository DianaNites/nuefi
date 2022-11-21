//! UEFI Device Path Protocol
use log::error;

use super::{Guid, Protocol, Str16};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
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
pub struct RawDevicePath {
    ty: u8,
    sub_ty: u8,
    /// Length, including this header
    len: [u8; 2],
}

interface!(DevicePath(RawDevicePath));

impl<'table> DevicePath<'table> {
    /// Convert this node to a UEFI String
    pub fn to_text(&self) -> Result<()> {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();
            if let Some(to) = boot.locate_protocol::<DevicePathToText>()? {
                //
            }
            todo!();
        } else {
            error!("Tried to use DevicePath::to_text while not in Boot mode");
            Err(UefiError::new(EfiStatus::UNSUPPORTED))
        }
    }
}

impl<'table> Drop for DevicePath<'table> {
    fn drop(&mut self) {
        if let Some(table) = get_boot_table() {
            let ret = unsafe { table.boot().free_pool(self.interface as *mut u8) };
            if ret.is_err() {
                error!("Failed to deallocate DevicePath {:p}", self.interface)
            }
        } else {
            error!(
                "Tried to deallocate DevicePath {:p} while not in Boot mode",
                self.interface
            )
        }
    }
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
#[derive(Debug)]
#[repr(C)]
pub struct RawDevicePathUtil {
    get_device_path_size: *mut u8,
    duplicate_device_path: *mut u8,
    append_device_path: *mut u8,
    append_device_node: *mut u8,
    append_device_path_instance: *mut u8,
    get_next_device_path_instance: *mut u8,
    is_device_path_multi_instance: *mut u8,
    create_device_node: *mut u8,
}

interface!(DevicePathUtil(RawDevicePathUtil));

unsafe impl<'table> Protocol<'table> for DevicePathUtil<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x37, 0x9b, 0xe4, 0xe, 0xd7, 0x06, 0x43, 0x7d, 0xb0, 0x37, 0xed, 0xb8, 0x2f, 0xb7,
            0x72, 0xa4,
        ])
    };

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { DevicePathUtil::new(this as *mut RawDevicePathUtil) }
    }
}

/// Device Path Display protocol
// #[derive(Debug)]
#[repr(C)]
pub struct RawDevicePathToText {
    convert_device_node_to_text: unsafe extern "efiapi" fn(
        node: *mut RawDevicePath,
        display: bool,
        shortcuts: bool,
    ) -> Str16,
    convert_device_path_to_text: unsafe extern "efiapi" fn(
        path: *mut RawDevicePath,
        display: bool,
        shortcuts: bool,
    ) -> Str16,
}

interface!(DevicePathToText(RawDevicePathToText));

impl<'table> DevicePathToText<'table> {
    ///
    pub fn convert_device_node_to_text(&self, node: &DevicePath) {
        unsafe { (self.interface().convert_device_node_to_text)(node.interface, false, false) };
    }

    ///
    pub fn convert_device_path_to_text(&self, path: &DevicePath) {
        unsafe { (self.interface().convert_device_path_to_text)(path.interface, false, false) };
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
