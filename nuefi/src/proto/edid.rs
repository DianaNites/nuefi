//! UEFI EDID Protocol services

use core::slice::from_raw_parts;

use crate::{interface, Protocol};

pub mod raw;
use raw::*;

interface!(
    #[Protocol("BD8C1056-9F36-44EC-92A8-A6337F817986")]
    ActiveEdid(RawEdidActive)
);

impl<'boot> ActiveEdid<'boot> {
    /// EDID information from the active display, or [`None`]
    pub fn edid(&self) -> Option<&[u8]> {
        let i = self.interface();
        let size = i.size as usize;
        let ptr = i.edid;
        if size != 0 && !ptr.is_null() {
            // Safety:
            // - EDID information is valid from firmware and read only.
            unsafe { Some(from_raw_parts(ptr, size)) }
        } else {
            None
        }
    }
}

// {0x1c0c34f6,0xd380,0x41fa,\
// {0xa0,0x49,0x8a,0xd0,0x6c,0x1a,0x66,0xaa}}

interface!(
    #[Protocol("1C0C34F6-D380-41FA-A049-8AD06C1A66AA")]
    DiscoveredEdid(RawEdidDiscovered)
);
