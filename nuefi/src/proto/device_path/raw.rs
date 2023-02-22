/// Raw device path structure
///
/// Device Paths are variable length, unaligned/byte packed structures.
///
/// All fields must be assumed unaligned
///
/// Also a protocol that can be used on any handle to obtain its path, if it
/// exists.
#[derive(Debug)]
#[repr(C, packed)]
pub struct RawDevicePath {
    pub ty: u8,
    pub sub_ty: u8,
    /// Length, including this header
    pub len: [u8; 2],
}

impl RawDevicePath {
    /// Create a new [RawDevicePath]
    ///
    /// # Safety
    ///
    /// It is up to you to make sure this is a valid node.
    pub unsafe fn create(ty: u8, sub_ty: u8, len: u16) -> Self {
        Self {
            ty,
            sub_ty,
            len: len.to_le_bytes(),
        }
    }

    /// Create the end of path node
    pub fn end() -> Self {
        Self {
            ty: 0x7F,
            sub_ty: 0xFF,
            len: 4u16.to_le_bytes(),
        }
    }
}

/// Device Path Utilities protocol
// #[derive(Debug)]
#[repr(C)]
pub struct RawDevicePathUtil {
    pub get_device_path_size: Option<unsafe extern "efiapi" fn(this: *mut RawDevicePath) -> usize>,
    pub duplicate_device_path: *mut u8,
    pub append_device_path: *mut u8,
    pub append_device_node: *mut u8,
    pub append_device_path_instance: *mut u8,
    pub get_next_device_path_instance: *mut u8,
    pub is_device_path_multi_instance: *mut u8,
    pub create_device_node: *mut u8,
}

/// Device Path Display protocol
// #[derive(Debug)]
#[repr(C)]
pub struct RawDevicePathToText {
    pub convert_device_node_to_text: Option<
        unsafe extern "efiapi" fn(
            node: *mut RawDevicePath,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,

    pub convert_device_path_to_text: Option<
        unsafe extern "efiapi" fn(
            path: *mut RawDevicePath,
            display: bool,
            shortcuts: bool,
        ) -> *mut u16,
    >,
}
