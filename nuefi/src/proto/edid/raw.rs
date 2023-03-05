//! Raw UEFI EDID Protocol types

/// Raw EDID_ACTIVE_PROTOCOL struct
#[derive(Debug)]
#[repr(C)]
pub struct RawEdidActive {
    pub size: u32,
    pub edid: *mut u8,
}

/// Raw EFI_EDID_DISCOVERED_PROTOCOL struct
#[derive(Debug)]
#[repr(C)]
pub struct RawEdidDiscovered {
    pub size: u32,
    pub edid: *mut u8,
}
