//! UEFI Media protocols
use crate::{
    proto::{Guid, Protocol},
    util::interface,
};

pub mod raw;
use raw::RawLoadFile2;

interface!(LoadFile2(RawLoadFile2));

impl<'table> LoadFile2<'table> {
    //
}

unsafe impl<'table> Protocol<'table> for LoadFile2<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x40, 0x06, 0xc0, 0xc1, 0xfc, 0xb3, 0x40, 0x3e, 0x99, 0x6d, 0x4a, 0x6c, 0x87, 0x24,
            0xe0, 0x6d,
        ])
    };

    type Raw = RawLoadFile2;

    unsafe fn from_raw(this: *mut RawLoadFile2) -> Self {
        unsafe { LoadFile2::new(this) }
    }
}
