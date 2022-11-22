//! Linux Specific UEFI Protocols

use crate::{
    proto::{device_path::DevicePath, Guid, Protocol},
    util::interface,
};

pub mod raw;
use raw::*;

interface!(InitrdMediaGuid(RawInitrdMediaGuid));

impl<'table> InitrdMediaGuid<'table> {
    pub fn as_device_path(&self) -> DevicePath {
        unsafe { DevicePath::from_raw(self.interface as *mut _) }
    }
}

unsafe impl<'table> Protocol<'table> for InitrdMediaGuid<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x55, 0x68, 0xe4, 0x27, 0x68, 0xfc, 0x4f, 0x3d, 0xac, 0x74, 0xca, 0x55, 0x52, 0x31,
            0xcc, 0x68,
        ])
    };

    type Raw = RawInitrdMediaGuid;

    unsafe fn from_raw(this: *mut RawInitrdMediaGuid) -> Self {
        unsafe { InitrdMediaGuid::new(this) }
    }
}
