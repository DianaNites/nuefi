//! UEFI Device Path Protocol
//!
//! This protocol is a bit special in that it points to an unaligned,
//! variable length binary structure identifying some specific device or
//! resource in a way consistent with the system topology.
//!
//! This Protocol should ideally be installed on device handles to indicate
//! their physical or logical device.
//!
//! UEFI Device Paths have different types, and each type has a different
//! sub-type, both of which together determine the data format for a
//! specific path node.
//!
//! # References
//!
//! - [UEFI Section 10. Device Path Protocol][s10]
//!
//! [s10]: <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html>

use core::ffi::c_void;

use nuefi_macros::GUID;

pub mod devpath_fn {
    //! Function definitions for [`super::DevicePathHdr`]
    //!
    //! # References
    //!
    //! - <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html>
    use super::DevicePathHdr;

    pub type GetDevicePathSize = unsafe extern "efiapi" fn(this: *mut DevicePathHdr) -> usize;

    pub type DuplicateDevicePath =
        unsafe extern "efiapi" fn(this: *mut DevicePathHdr) -> *mut DevicePathHdr;

    pub type AppendDeviceNode = unsafe extern "efiapi" fn(
        this: *mut DevicePathHdr,
        other: *mut DevicePathHdr,
    ) -> *mut DevicePathHdr;

    pub type AppendDevicePath = unsafe extern "efiapi" fn(
        this: *mut DevicePathHdr,
        other: *mut DevicePathHdr,
    ) -> *mut DevicePathHdr;

    pub type ConvertDeviceNodeToText = unsafe extern "efiapi" fn(
        node: *mut DevicePathHdr,
        display: bool,
        shortcuts: bool,
    ) -> *mut u16;

    pub type ConvertDevicePathToText = unsafe extern "efiapi" fn(
        path: *mut DevicePathHdr,
        display: bool,
        shortcuts: bool,
    ) -> *mut u16;
}

mod imp {
    //! Privately implement [`DevicePath`][`super::DevicePathHdr`]
    // use super::*;
}

pub mod nodes;
pub mod types;

use types::*;

use self::nodes::End;

/// Generic [`DevicePathHdr`] structure, and a
/// [`Protocol`][`crate::extra::Protocol`]
///
/// See [the module][`super::device_path`] docs for detail on what a
/// Device Path is
///
/// This protocol can be requested from any handle to obtain the path to
/// its physical/logical device.
///
/// # References
///
/// - [Section 10.2. EFI Device Path Protocol][s10_2]
///
/// [s10_2]: <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html#efi-device-path-protocol>
#[GUID("09576E91-6D3F-11D2-8E39-00A0C969723B")]
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DevicePathHdr {
    /// Type of device path
    pub ty: DevicePathType,

    /// Type specific sub-type
    pub sub_ty: DevicePathSubType,

    /// Length, in ***bytes***, including this header
    pub len: [u8; 2],
}

impl DevicePathHdr {
    /// Create a new [`DevicePathHdr`]
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
        // Safety: Identical layouts
        unsafe { core::mem::transmute(End::entire()) }
    }

    /// Create a media filepath node for a null terminated path of bytes `len`
    pub fn media_file(len: u16) -> Self {
        let len = len.checked_add(4).unwrap();
        Self {
            ty: DevicePathType::MEDIA,
            sub_ty: sub::media::FILE,
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
    pub convert_device_node_to_text: Option<devpath_fn::ConvertDeviceNodeToText>,

    pub convert_device_path_to_text: Option<devpath_fn::ConvertDevicePathToText>,
}
