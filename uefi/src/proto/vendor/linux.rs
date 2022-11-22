//! Linux Specific UEFI Protocols
use log::{error, info, trace};

use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    proto::{
        device_path::{DevicePath, RawDevicePath},
        Guid,
        Protocol,
        Str16,
    },
    string::{string_len, UefiString},
    util::interface,
};

/// The linux specific EFI_INITRD_MEDIA_GUID protocol
///
/// Poorly non-documented [here][1].
///
/// This is a vendor defined media [DevicePath].
///
/// Linux for some reason appears to use this as a marker.
/// It is just a [DevicePath].
///
/// The real thing is implemented by the LoadFile2 protocol apparently
///
/// [1]: https://docs.kernel.org/x86/boot.html#efi-handover-protocol-deprecated
#[repr(C, packed)]
pub struct RawInitrdMediaGuid {
    path: RawDevicePath,
    guid: [u8; 16],
    end: RawDevicePath,
}

impl RawInitrdMediaGuid {
    /// Create a new instance of this protocol
    pub fn create() -> Self {
        Self {
            path: unsafe { RawDevicePath::create(4, 3, 20) },
            guid: InitrdMediaGuid::GUID.0,
            end: RawDevicePath::end(),
        }
    }
}

interface!(InitrdMediaGuid(RawInitrdMediaGuid));

impl<'table> InitrdMediaGuid<'table> {
    pub fn as_device_path(&mut self) -> DevicePath {
        unsafe { DevicePath::from_raw(self as *mut _ as *mut u8) }
    }
}

unsafe impl<'table> Protocol<'table> for InitrdMediaGuid<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x55, 0x68, 0xe4, 0x27, 0x68, 0xfc, 0x4f, 0x3d, 0xac, 0x74, 0xca, 0x55, 0x52, 0x31,
            0xcc, 0x68,
        ])
    };

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { InitrdMediaGuid::new(this as *mut RawInitrdMediaGuid) }
    }
}
