/// [`RawDevicePath`] types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RawDevicePathType(u8);

impl RawDevicePathType {
    /// Represents a device connected to the system
    pub const HARDWARE: Self = Self(0x01);

    /// Represents ACPI Plug and Play hardware?
    pub const ACPI: Self = Self(0x02);

    /// Represents the connection to a device on another system,
    /// such as a IP address or SCSI ID.
    pub const MESSAGING: Self = Self(0x03);

    /// Represents the portion of an entity that is being abstracted,
    /// such as a file path on a storage device.
    pub const MEDIA: Self = Self(0x04);

    /// Used by platform firmware to select legacy bios boot options
    pub const BIOS: Self = Self(0x05);

    /// Represents the end of the device path
    pub const END: Self = Self(0x7F);
}

/// [`RawDevicePath`] Sub Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RawDevicePathSubType(u8);

impl RawDevicePathSubType {
    /// Represents a file [Media][`RawDevicePathType::MEDIA`] path
    pub const MEDIA_FILE: Self = Self(0x04);

    /// Represents the end of the entire [`RawDevicePath`]
    pub const END_ENTIRE: Self = Self(0xFF);

    /// Represents the end of this [`RawDevicePath`] instance
    /// and the start of a new one
    pub const END_INSTANCE: Self = Self(0x01);
}

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
    pub ty: RawDevicePathType,

    pub sub_ty: RawDevicePathSubType,

    /// Length, in ***bytes***, including this header
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
            ty: RawDevicePathType(ty),
            sub_ty: RawDevicePathSubType(sub_ty),
            // Note: These are always LE integers?
            len: len.to_le_bytes(),
        }
    }

    /// Create the end of path node
    pub fn end() -> Self {
        Self {
            ty: RawDevicePathType::END,
            sub_ty: RawDevicePathSubType::END_ENTIRE,
            // Note: These are always LE integers?
            len: 4u16.to_le_bytes(),
        }
    }

    /// Create a media filepath node for a null terminated path of bytes `len`
    pub fn media_file(len: u16) -> Self {
        let len = len.checked_add(4).unwrap();
        Self {
            ty: RawDevicePathType::MEDIA,
            sub_ty: RawDevicePathSubType::MEDIA_FILE,
            // Note: These are always LE integers?
            len: len.to_le_bytes(),
        }
    }
}

pub type GetDevicePathSize = unsafe extern "efiapi" fn(this: *mut RawDevicePath) -> usize;

pub type DuplicateDevicePath =
    unsafe extern "efiapi" fn(this: *mut RawDevicePath) -> *mut RawDevicePath;

pub type AppendDeviceNode = unsafe extern "efiapi" fn(
    this: *mut RawDevicePath,
    other: *mut RawDevicePath,
) -> *mut RawDevicePath;

pub type AppendDevicePath = unsafe extern "efiapi" fn(
    this: *mut RawDevicePath,
    other: *mut RawDevicePath,
) -> *mut RawDevicePath;

/// Device Path Utilities protocol
// #[derive(Debug)]
#[repr(C)]
pub struct RawDevicePathUtil {
    pub get_device_path_size: Option<GetDevicePathSize>,
    pub duplicate_device_path: Option<DuplicateDevicePath>,
    pub append_device_path: Option<AppendDevicePath>,
    pub append_device_node: Option<AppendDeviceNode>,
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
