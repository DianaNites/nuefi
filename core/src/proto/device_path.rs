//! UEFI Device Path Protocol
//!
//! A UEFI Device Path is a variable length unaligned packed binary structure.
//!
//! # References
//!
//! - [UEFI Section 10. Device Path Protocol][s10]
//!
//! [s10]: <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html>

use core::ffi::c_void;

use nuefi_macros::GUID;

pub mod devpath_fn {
    //! Function definitions for [`super::DevicePath`]
    //!
    //! # References
    //!
    //! - <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html>
    use super::DevicePath;

    pub type GetDevicePathSize = unsafe extern "efiapi" fn(this: *mut DevicePath) -> usize;

    pub type DuplicateDevicePath =
        unsafe extern "efiapi" fn(this: *mut DevicePath) -> *mut DevicePath;

    pub type AppendDeviceNode =
        unsafe extern "efiapi" fn(this: *mut DevicePath, other: *mut DevicePath) -> *mut DevicePath;

    pub type AppendDevicePath =
        unsafe extern "efiapi" fn(this: *mut DevicePath, other: *mut DevicePath) -> *mut DevicePath;
}

mod imp {
    //! Privately implement [`DevicePath`][super::DevicePath]
    // use super::*;
}

/// [`DevicePath`] types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DevicePathType(u8);

impl DevicePathType {
    /// Represents a device connected to the system
    pub const HARDWARE: Self = Self(0x01);

    /// Represents ACPI Plug and Play hardware?
    pub const ACPI: Self = Self(0x02);

    /// Represents the connection to a device on another system,
    /// such as a IP address or SCSI ID.
    pub const MESSAGING: Self = Self(0x03);

    /// Represents the portion of an entity that is being abstracted,
    /// such as a file path on a storage device.
    pub const MEDIA: Self = Self(0x04);

    /// Used by platform firmware to select legacy bios boot options
    pub const BIOS: Self = Self(0x05);

    /// Represents the end of the device path
    pub const END: Self = Self(0x7F);
}

/// [`DevicePath`] Sub Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DevicePathSubType(u8);

impl DevicePathSubType {
    /// Represents a file [Media][`DevicePathType::MEDIA`] path
    pub const MEDIA_FILE: Self = Self(0x04);

    /// Represents the end of the entire [`DevicePath`]
    pub const END_ENTIRE: Self = Self(0xFF);

    /// Represents the end of this [`DevicePath`] instance
    /// and the start of a new one
    pub const END_INSTANCE: Self = Self(0x01);
}

/// Generic [`DevicePath`] structure, and a [`Protocol`][super::Protocol]
///
/// See [the module][`super::device_path`] docs for detail on what a Device Path
/// is
///
/// This protocol can be requested from any handle to obtain the path to
/// its physical/logical device.
///
/// # References
///
/// - [Section 10.2. EFI Device Path Protocol][s10_2]
///
/// [s10_2]: <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html#efi-device-path-protocol>
#[GUID("09576E91-6D3F-11D2-8E39-00A0C969723B", crate("crate"))]
#[derive(Debug)]
#[repr(C, packed)]
pub struct DevicePath {
    pub ty: DevicePathType,

    pub sub_ty: DevicePathSubType,

    /// Length, in ***bytes***, including this header
    pub len: [u8; 2],
}

impl DevicePath {
    /// Create a new [`DevicePath`]
    ///
    /// # Safety
    ///
    /// It is up to you to make sure this is a valid node.
    pub unsafe fn create(ty: u8, sub_ty: u8, len: u16) -> Self {
        Self {
            ty: DevicePathType(ty),
            sub_ty: DevicePathSubType(sub_ty),
            len: len.to_le_bytes(),
        }
    }

    /// Create the end of path node
    pub fn end() -> Self {
        Self {
            ty: DevicePathType::END,
            sub_ty: DevicePathSubType::END_ENTIRE,
            len: 4u16.to_le_bytes(),
        }
    }

    /// Create a media filepath node for a null terminated path of bytes `len`
    pub fn media_file(len: u16) -> Self {
        let len = len.checked_add(4).unwrap();
        Self {
            ty: DevicePathType::MEDIA,
            sub_ty: DevicePathSubType::MEDIA_FILE,
            len: len.to_le_bytes(),
        }
    }
}

/// Device Path Utilities protocol
// #[derive(Debug)]
#[repr(C)]
pub struct DevicePathUtil {
    pub get_device_path_size: Option<devpath_fn::GetDevicePathSize>,
    pub duplicate_device_path: Option<devpath_fn::DuplicateDevicePath>,
    pub append_device_path: Option<devpath_fn::AppendDevicePath>,
    pub append_device_node: Option<devpath_fn::AppendDeviceNode>,
    pub append_device_path_instance: *mut c_void,
    pub get_next_device_path_instance: *mut c_void,
    pub is_device_path_multi_instance: *mut c_void,
    pub create_device_node: *mut c_void,
}

/// Device Path Display protocol
// #[derive(Debug)]
#[repr(C)]
pub struct DevicePathToText {
    pub convert_device_node_to_text: Option<
        unsafe extern "efiapi" fn(
            node: *mut DevicePath,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,

    pub convert_device_path_to_text: Option<
        unsafe extern "efiapi" fn(
            path: *mut DevicePath,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,
}
