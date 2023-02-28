//! Linux Specific UEFI Protocols

use raw::*;

use crate::{
    proto::{device_path::DevicePath, Guid, Protocol},
    util::interface,
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("5568E427-68FC-4F3D-AC74-CA555231CC68", crate("crate"))]
    InitrdMediaGuid(RawInitrdMediaGuid)
);

impl<'table> InitrdMediaGuid<'table> {
    pub fn as_device_path(&self) -> DevicePath {
        // Safety: This is just a specific variant of a generic DevicePath
        // FIXME: This should probably return a reference
        // It would be safe to cast `&self` to `&DevicePath` because we know their
        // layouts, and they're transparent.
        unsafe { DevicePath::from_raw(self.interface as *mut _) }
    }
}
