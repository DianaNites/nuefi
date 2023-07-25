use nuefi_core::proto::device_path::nodes::{media::Vendor, End};

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
/// [1]: https://www.kernel.org/doc/html/latest/arch/x86/boot.html?highlight=boot#efi-handover-protocol-deprecated
#[repr(C, packed)]
pub struct RawInitrdMediaGuid {
    pub vendor: Vendor,
    pub end: End,
}

impl RawInitrdMediaGuid {
    /// Create a new instance of this protocol
    pub fn create() -> Self {
        Self {
            vendor: Vendor::new_header(InitrdMediaGuid::GUID, 0),
            end: End::entire(),
        }
    }
}
