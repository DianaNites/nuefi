//! UEFI Device Path Protocol
//!
//! This protocol is special in that it is not actually a protocol,
//! but a unaligned, variable length binary structure.
//!
//! Like a Protocol, a Device Path has a GUID, and the UEFI specification
//! refers to it as a Protocol, but it has no methods.
//!
//! UEFI Device Paths have different types, and each type has a different
//! sub-type, both of which together determine the daa format for a path node.
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
}

mod imp {
    //! Privately implement [`DevicePath`][`super::DevicePathHdr`]
    // use super::*;
}

/// [`DevicePathHdr`] types
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

    /// Represents the end of the device path structure
    pub const END: Self = Self(0x7F);
}

/// [`DevicePathHdr`] Sub Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DevicePathSubType(u8);

impl DevicePathSubType {
    /// Represents a file [Media][`DevicePathType::MEDIA`] path
    pub const MEDIA_FILE: Self = Self(0x04);

    /// Represents the end of the entire [`DevicePathHdr`]
    pub const END_ENTIRE: Self = Self(0xFF);

    /// Represents the end of this [`DevicePathHdr`] instance
    /// and the start of a new one
    pub const END_INSTANCE: Self = Self(0x01);
}

/// Generic [`DevicePathHdr`] structure, and a
/// [`Protocol`][`crate::extra::Protocol`]
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
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DevicePathHdr {
    pub ty: DevicePathType,

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
            node: *mut DevicePathHdr,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,

    pub convert_device_path_to_text: Option<
        unsafe extern "efiapi" fn(
            path: *mut DevicePathHdr,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,
}

/// A generic UEFI Device Path
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GenericDevicePath {
    hdr: DevicePathHdr,
}

impl GenericDevicePath {
    /// # Safety
    pub const unsafe fn from_raw<'a>(_hdr: *const DevicePathHdr) -> &'a GenericDevicePath {
        todo!()
    }
}
