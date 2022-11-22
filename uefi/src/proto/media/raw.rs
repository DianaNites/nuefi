use crate::{error::EfiStatus, proto::device_path::raw::RawDevicePath};

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
    pub load_file: Option<LoadFile2Fn>,
}

impl RawLoadFile2 {
    /// Create a new instance of this protocol
    pub fn create(load_file: LoadFile2Fn) -> Self {
        Self {
            load_file: Some(load_file),
        }
    }
}
