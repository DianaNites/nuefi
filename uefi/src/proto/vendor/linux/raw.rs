use super::InitrdMediaGuid;
use crate::proto::{device_path::raw::RawDevicePath, Protocol};

/// The linux specific EFI_INITRD_MEDIA_GUID protocol
///
/// Poorly non-documented [here][1].
///
/// This is a vendor defined media [crate::proto::device_path::DevicePath].
///
/// Linux for some reason appears to use this as a marker.
/// It is just a [crate::proto::device_path::DevicePath].
///
/// The real thing is implemented by the LoadFile2 protocol apparently
///
/// [1]: https://docs.kernel.org/x86/boot.html#efi-handover-protocol-deprecated
#[repr(C, packed)]
pub struct RawInitrdMediaGuid {
    pub path: RawDevicePath,
    pub guid: [u8; 16],
    pub end: RawDevicePath,
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
