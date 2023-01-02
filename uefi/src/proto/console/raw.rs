//! Raw UEFI data types

use core::fmt;

use super::Str16;
use crate::error::EfiStatus;

#[derive(Debug)]
#[repr(C)]
pub struct RawSimpleTextInput {
    //
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawMode {
    pub max_mode: i32,
    pub mode: i32,
    pub attribute: i32,
    pub cursor_column: i32,
    pub cursor_row: i32,
    pub cursor_visible: bool,
}

// TODO: Report bug to upstream Rust that derive(Debug) doesn't work for efiapi
// #[derive(Debug)]
#[repr(C)]
pub struct RawSimpleTextOutput {
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> EfiStatus,
    pub output_string: unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus,
    pub test_string: unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus,
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode: usize,
        cols: *mut usize,
        rows: *mut usize,
    ) -> EfiStatus,
    pub set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode: usize) -> EfiStatus,
    pub set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attr: usize) -> EfiStatus,
    pub clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> EfiStatus,
    pub set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, cols: usize, rows: usize) -> EfiStatus,
    pub enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> EfiStatus,
    pub mode: *mut RawMode,
}

/// EFI Physical Address
///
/// Defined at https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#efi-boot-services-allocatepages
pub type EfiPhysicalAddress = u64;

/// Read only structure defining information about available video modes
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawGraphicsMode {
    /// Modes supported by UEFI
    ///
    /// Valid mode numbers are `0` to `max_mode - 1`
    pub max_mode: u32,

    /// Currently selected mode
    ///
    /// Valid mode numbers are `0` to `max_mode - 1`
    pub mode: u32,

    /// Pointer to read only info structure
    pub info: *const RawGraphicsInfo,

    /// Size of the info structure
    pub info_size: usize,

    /// Pointer to framebuffer
    pub fb_base: EfiPhysicalAddress,

    /// Size to framebuffer
    pub fb_size: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawGraphicsInfo {
    /// Version of this structure
    ///
    /// Currently zero as of UEFI 2.10
    pub version: u32,

    /// Horizontal resolution
    pub horizontal: u32,

    /// Vertical resolution
    pub vertical: u32,

    /// Physical pixel format
    pub format: RawPixelFormat,

    /// Only valid if `format` is set to [`RawPixelFormat::BIT_MASK`]
    pub info: RawPixelMask,

    /// Defines padding pixels between video memory line, outside `horizontal`
    pub stride: u32,
}

/// Bits set here define the bits making up a pixel
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawPixelMask {
    pub red: u32,
    pub green: u32,
    pub blue: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RawPixelFormat(u32);

impl fmt::Debug for RawPixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::RGB => write!(f, "RawPixelFormat::RGB"),
            Self::BGR => write!(f, "RawPixelFormat::BGR"),
            Self::BIT_MASK => write!(f, "RawPixelFormat::BIT_MASK"),
            Self::BLT_ONLY => write!(f, "RawPixelFormat::BLT_ONLY"),
            Self::FORMAT_MAX => write!(f, "RawPixelFormat::FORMAT_MAX"),
            _ => f.debug_tuple("RawPixelFormat").field(&self.0).finish(),
        }
    }
}

impl RawPixelFormat {
    /// RBG Pixels
    pub const RGB: Self = Self(0);

    /// BGR Pixels
    pub const BGR: Self = Self(1);

    /// Pixels defined by [`RawPixelMask`]
    pub const BIT_MASK: Self = Self(2);

    /// Only blt supported, no framebuffer
    pub const BLT_ONLY: Self = Self(3);

    /// Current max enum value
    pub const FORMAT_MAX: Self = Self(4);
}

/// UEFI Graphics Output Protocol
///
/// https://uefi.org/specs/UEFI/2.10/12_Protocols_Console_Support.html#graphics-output-protocol
#[repr(C)]
pub struct RawGraphicsOutput {
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode: u32,
        info_size: *mut usize,
        info: *mut *const RawGraphicsInfo,
    ) -> EfiStatus,
    pub set_mode: unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> EfiStatus,
    pub blt: unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> EfiStatus,
    pub mode: *mut RawGraphicsMode,
}
