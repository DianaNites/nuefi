//! UEFI Media protocols
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    proto::{
        self,
        device_path::{DevicePath, RawDevicePath},
        Guid,
        Protocol,
        Str16,
    },
    string::{string_len, UefiString},
    util::interface,
    EfiHandle,
};

pub type LoadFile2Fn = unsafe extern "efiapi" fn(
    this: *mut RawLoadFile2,
    path: *mut RawDevicePath,
    boot: bool,
    buf_size: *mut usize,
    buf: *mut u8,
) -> EfiStatus;

/// UEFI LoadFile2 protocol
#[repr(C)]
pub struct RawLoadFile2 {
    load_file: Option<LoadFile2Fn>,
}

impl RawLoadFile2 {
    /// Create a new instance of this protocol
    pub fn create(load_file: LoadFile2Fn) -> Self {
        Self {
            load_file: Some(load_file),
        }
    }
}

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

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { LoadFile2::new(this as *mut RawLoadFile2) }
    }
}
