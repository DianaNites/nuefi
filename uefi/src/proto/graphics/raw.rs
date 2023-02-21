//! Raw Graphics types

use core::fmt;

use super::Str16;
use crate::error::EfiStatus;

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
    pub query_mode: Option<
        unsafe extern "efiapi" fn(
            this: *mut Self,
            mode: u32,
            info_size: *mut usize,
            info: *mut *const RawGraphicsInfo,
        ) -> EfiStatus,
    >,

    pub set_mode: Option<unsafe extern "efiapi" fn(this: *mut Self, mode: u32) -> EfiStatus>,

    pub blt: Option<
        unsafe extern "efiapi" fn(
            //
            this: *mut Self,
            buffer: *mut RawBltPixel,
            op: RawBltOperation,
            src_x: usize,
            src_y: usize,
            dest_x: usize,
            dest_y: usize,
            width: usize,
            height: usize,
            delta: usize,
        ) -> EfiStatus,
    >,

    pub mode: *mut RawGraphicsMode,
}

impl fmt::Debug for RawGraphicsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawGraphicsOutput")
            .field("query_mode", &{ &self.query_mode as *const _ })
            .field("set_mode", &{ &self.set_mode as *const _ })
            .field("blt", &{ &self.blt as *const _ })
            .field("mode", &self.mode)
            .finish()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RawBltPixel {
    blue: u8,
    green: u8,
    red: u8,
    reserved: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RawBltOperation(u32);

impl fmt::Debug for RawBltOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::VIDEO_FILL => write!(f, "RawBltOperation::VIDEO_FILL"),
            Self::VIDEO_TO_BUFFER => write!(f, "RawBltOperation::VIDEO_TO_BUFFER"),
            Self::BUFFER_TO_VIDEO => write!(f, "RawBltOperation::BUFFER_TO_VIDEO"),
            Self::VIDEO_TO_VIDEO => write!(f, "RawBltOperation::VIDEO_TO_VIDEO"),
            Self::OPERATION_MAX => write!(f, "RawBltOperation::OPERATION_MAX"),
            _ => f.debug_tuple("RawBltOperation").field(&self.0).finish(),
        }
    }
}

impl RawBltOperation {
    /// Write data from the 0th buffer pixel to every pixel in the block
    pub const VIDEO_FILL: Self = Self(0);

    /// Read data from video block to buffer block
    pub const VIDEO_TO_BUFFER: Self = Self(1);

    /// Write data from block buffer to video buffer
    pub const BUFFER_TO_VIDEO: Self = Self(2);

    /// Copy data from source block to destination block
    pub const VIDEO_TO_VIDEO: Self = Self(3);

    /// Current max enum value
    pub const OPERATION_MAX: Self = Self(4);
}

impl RawBltOperation {
    pub(crate) fn new(value: u32) -> RawBltOperation {
        Self(value)
    }
}
