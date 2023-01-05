//! UEFI Graphics related protocols
use core::{
    fmt::{self, Write},
    iter::once,
    marker::PhantomData,
    mem::size_of,
    ops::{Index, IndexMut},
    slice::{from_raw_parts, from_raw_parts_mut},
};

use super::{Guid, Str16};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    util::interface,
};

use alloc::vec::Vec;
use raw::{RawBltOperation, RawBltPixel, RawGraphicsInfo, RawGraphicsOutput, RawPixelFormat};

pub mod raw;

interface!(GraphicsOutput(RawGraphicsOutput));

impl<'table> GraphicsOutput<'table> {
    /// Set the graphic mode to number `mode`
    pub fn set_mode(&self, mode: u32) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().set_mode)(self.interface, mode) }.into()
    }

    pub fn query_mode(&self, mode: u32) -> Result<GraphicsMode> {
        let mut size = 0;
        let mut info = core::ptr::null();
        // Safety: Construction ensures these are valid
        let ret = unsafe {
            //
            (self.interface().query_mode)(self.interface, mode, &mut size, &mut info)
        };
        if ret.is_success() && !info.is_null() && size >= size_of::<RawGraphicsInfo>() {
            let mode = GraphicsMode::new(
                mode,
                // Safety: Checked for null and size above
                unsafe { *info },
            );
            if let Some(table) = get_boot_table() {
                // Safety: `info` was allocated by UEFI
                unsafe {
                    table.boot().free_pool(info as *mut u8)?;
                }
            }
            Ok(mode)
        } else if !ret.is_success() {
            Err(UefiError::new(ret))
        } else {
            Err(UefiError::new(EfiStatus::BUFFER_TOO_SMALL))
        }
    }

    pub fn modes(&self) -> impl Iterator<Item = Result<GraphicsMode>> + '_ {
        let mut mode = 0;
        core::iter::from_fn(move || {
            if mode >= self.max_mode() {
                return None;
            }
            let m = self.query_mode(mode);
            mode += 1;
            Some(m)
        })
    }

    /// Current [`GraphicsMode`]
    pub fn mode(&self) -> GraphicsMode {
        // Safety: types
        let info = unsafe { *(*self.interface().mode).info };
        // Safety: types
        let mode = unsafe { (*self.interface().mode).mode };
        GraphicsMode::new(mode, info)
    }

    /// Blt, or BLock Transfer
    ///
    /// (x, y)
    /// (width, height)
    ///
    /// `buffer` must be at least `width * height`
    /// or else `INVALID_PARAMETER` will be returned.
    ///
    /// If the width in `buffer` is not the same as the display then
    /// `delta` must contain the data width (pixels) or else output will be garbled.
    ///
    /// Buffer is BGR formatted 32-bit pixels
    pub fn blt(
        &self,
        buffer: &[Pixel],
        op: BltOperation,
        src: (usize, usize),
        dest: (usize, usize),
        res: (usize, usize),
        delta: usize,
    ) -> Result<()> {
        if buffer.len() < (res.0 * res.1) {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().blt)(
                self.interface,
                buffer.as_ptr() as *mut RawBltPixel,
                op.into(),
                src.0,
                src.1,
                dest.0,
                dest.1,
                res.0,
                res.1,
                delta * size_of::<RawBltPixel>(),
            )
        }
        .into()
    }

    /// Get a mutable byte slice to the current framebuffer
    ///
    /// Note that each pixel `(x, y)`
    /// is at the `<size of a pixel> *`[`GraphicsMode::stride`]
    pub fn framebuffer(&self) -> Result<Framebuffer<'_>> {
        // FIXME: Volatile?
        // Safety:
        unsafe {
            let mode = &*self.interface().mode;
            let ptr = mode.fb_base as *mut u8;
            let size = mode.fb_size;
            let fb = Framebuffer::new(ptr, size, self.mode().stride());
            Ok(fb)
        }
    }

    fn max_mode(&self) -> u32 {
        // Safety: Type system
        unsafe { (*self.interface().mode).max_mode }
    }
}

#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'table> super::Protocol<'table> for GraphicsOutput<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x90, 0x42, 0xa9, 0xde, 0x23, 0xdc, 0x4a, 0x38, 0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80,
            0x51, 0x6a,
        ])
    };

    type Raw = RawGraphicsOutput;

    unsafe fn from_raw(this: *mut Self::Raw) -> Self {
        GraphicsOutput::new(this)
    }
}

/// UEFI Graphics Mode Information
#[derive(Debug)]
pub struct GraphicsMode {
    /// Mode number
    mode: u32,

    info: RawGraphicsInfo,
}

impl GraphicsMode {
    fn new(mode: u32, info: RawGraphicsInfo) -> Self {
        Self { mode, info }
    }

    /// UEFI Framebuffer (horizontal, vertical) resolution
    ///
    /// Otherwise known as (width, height)
    pub fn res(&self) -> (u32, u32) {
        (self.info.horizontal, self.info.vertical)
    }

    /// UEFI Framebuffer stride
    pub fn stride(&self) -> u32 {
        self.info.stride
    }

    /// Mode number
    pub fn mode(&self) -> u32 {
        self.mode
    }

    /// Pixel Format
    pub fn format(&self) -> PixelFormat {
        self.info.format.into()
    }
}

/// UEFI Framebuffer pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PixelFormat {
    /// RBG Pixels
    RGB,

    /// BGR Pixels
    BGR,

    /// Pixels defined by [`RawPixelMask`]
    BitMask,

    /// Only blt supported, no framebuffer
    BltOnly,
}

impl From<RawPixelFormat> for PixelFormat {
    fn from(value: RawPixelFormat) -> Self {
        match value {
            RawPixelFormat::RGB => PixelFormat::RGB,
            RawPixelFormat::BGR => PixelFormat::BGR,
            RawPixelFormat::BIT_MASK => PixelFormat::BitMask,
            RawPixelFormat::BLT_ONLY => PixelFormat::BltOnly,
            _ => unimplemented!(),
        }
    }
}

/// UEFI Framebuffer pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[repr(u32)]
pub enum BltOperation {
    /// Write data from the 0th buffer pixel to every pixel in the block
    VideoFill,

    /// Read data from video block to buffer block
    VideoToBuffer,

    /// Write data from block buffer to video buffer
    BufferToVideo,

    /// Copy data from source block to destination block
    VideoToVideo,
}

impl From<RawBltOperation> for BltOperation {
    fn from(value: RawBltOperation) -> Self {
        match value {
            RawBltOperation::VIDEO_FILL => BltOperation::VideoFill,
            RawBltOperation::VIDEO_TO_BUFFER => BltOperation::VideoToBuffer,
            RawBltOperation::BUFFER_TO_VIDEO => BltOperation::BufferToVideo,
            RawBltOperation::VIDEO_TO_VIDEO => BltOperation::VideoToVideo,
            _ => unimplemented!(),
        }
    }
}

impl From<BltOperation> for RawBltOperation {
    fn from(value: BltOperation) -> Self {
        RawBltOperation::new(value as u32)
    }
}

/// UEFI Framebuffer
#[derive(Debug)]
pub struct Framebuffer<'mode> {
    ptr: *mut u8,
    size: usize,
    stride: u32,
    phantom: PhantomData<&'mode u8>,
}

impl<'mode> Framebuffer<'mode> {
    /// Create new Framebuffer wrapper
    ///
    /// - `ptr` MUST be valid for `size` bytes
    unsafe fn new(ptr: *mut u8, size: usize, stride: u32) -> Self {
        Self {
            ptr,
            size,
            stride,
            phantom: PhantomData,
        }
    }

    pub fn pixels(&self) -> &'mode [Pixel] {
        let ptr = self.ptr as *mut Pixel;
        let len = self.size / size_of::<Pixel>();
        // Safety:
        unsafe { from_raw_parts(ptr, len) }
    }

    pub fn pixels_mut(&mut self) -> &'mode mut [Pixel] {
        let ptr = self.ptr as *mut Pixel;
        let len = self.size / size_of::<Pixel>();
        // Safety:
        unsafe { from_raw_parts_mut(ptr, len) }
    }
}

impl<'m> Index<(u32, u32)> for Framebuffer<'m> {
    type Output = Pixel;

    fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
        let index = ((y * self.stride) + x) as usize;
        // Safety:
        unsafe {
            //
            &*self.ptr.add(index).cast::<Pixel>()
        }
    }
}

/// A UEFI BGR888 Pixel
///
/// 32-bits in size, 24-bits usable
///
/// ABI Compatible with `[u8; 4]`
#[derive(Debug, Clone, Copy, Default)]
#[repr(transparent)]
pub struct Pixel {
    data: [u8; 4],
}

impl Pixel {
    /// Create a new pixel
    ///
    /// # Note
    ///
    /// Takes arguments in RGB order for convenience
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { data: [b, g, r, 0] }
    }

    /// Create a new pixel using `data` as-is
    ///
    /// Data is assumed to be BGR888
    pub fn from_bytes(data: [u8; 4]) -> Self {
        Self { data }
    }

    /// Get an array with each 8 bit color component in a byte
    ///
    /// Last byte is always 0
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.data
    }

    pub fn red(&self) -> u8 {
        self.data[2]
    }

    pub fn green(&self) -> u8 {
        self.data[1]
    }

    pub fn blue(&self) -> u8 {
        self.data[0]
    }
}

/// UEFI Pixel Coordinate
#[derive(Debug, Clone, Copy)]
#[repr(C)]
// Stores x, y coordinates
pub struct Coord(u32, u32);

impl Coord {
    pub fn new(x: u32, y: u32) -> Self {
        Self(x, y)
    }
}
