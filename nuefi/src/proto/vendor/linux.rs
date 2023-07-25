//! Linux Specific UEFI Protocols

use raw::*;

use crate::{
    nuefi_core::interface,
    proto::{device_path::DevicePath, Guid, Protocol},
    GUID,
};

pub mod raw;

interface!(
    /// Device Path defined by Linux identifying a handle that supports the
    /// [`media::LoadFile2`] [`Protocol`] for loading the initial ram disk
    #[GUID("5568E427-68FC-4F3D-AC74-CA555231CC68")]
    InitrdMediaGuid(RawInitrdMediaGuid)
);

impl<'table> InitrdMediaGuid<'table> {
    pub fn as_device_path(&self) -> DevicePath<'_> {
        // Safety: This is a DevicePath
        unsafe { DevicePath::from_raw(self.interface as *mut _) }
    }
}
