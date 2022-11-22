//! UEFI PI Security Protocols

use crate::{
    proto::{Guid, Protocol},
    util::interface,
};

pub mod raw;
use raw::{RawSecurityArch, RawSecurityArch2};

interface!(SecurityArch(RawSecurityArch));

impl<'table> SecurityArch<'table> {
    //
}

unsafe impl<'table> Protocol<'table> for SecurityArch<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0xA4, 0x64, 0x23, 0xE3, 0x46, 0x17, 0x49, 0xf1, 0xB9, 0xFF, 0xD1, 0xBF, 0xA9, 0x11,
            0x58, 0x39,
        ])
    };

    type Raw = RawSecurityArch;

    unsafe fn from_raw(this: *mut RawSecurityArch) -> Self {
        unsafe { SecurityArch::new(this) }
    }
}

interface!(SecurityArch2(RawSecurityArch2));

impl<'table> SecurityArch2<'table> {
    //
}

unsafe impl<'table> Protocol<'table> for SecurityArch2<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x94, 0xab, 0x2f, 0x58, 0x14, 0x38, 0x4e, 0xf1, 0x91, 0x52, 0x18, 0x94, 0x1a, 0x3a,
            0xe, 0x68,
        ])
    };

    type Raw = RawSecurityArch2;

    unsafe fn from_raw(this: *mut RawSecurityArch2) -> Self {
        unsafe { SecurityArch2::new(this) }
    }
}
