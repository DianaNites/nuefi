use crate::{
    mem::MemoryType,
    proto::device_path::raw::RawDevicePath,
    table::raw::RawSystemTable,
    EfiHandle,
};

/// Raw UEFI LoadedImage protocol structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawLoadedImage {
    pub revision: u32,
    pub parent: EfiHandle,
    pub system_table: *mut RawSystemTable,

    pub device: EfiHandle,
    pub path: *mut RawDevicePath,
    pub _reserved: *mut u8,

    pub options_size: u32,
    pub options: *mut u8,

    pub image_base: *mut u8,
    pub image_size: u64,
    pub image_code: MemoryType,
    pub image_data: MemoryType,
    pub unload: *mut u8,
}
