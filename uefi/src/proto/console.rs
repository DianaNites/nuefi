//! UEFI Console related protocols
use core::{
    fmt::{self, Write},
    iter::once,
    mem::size_of,
    slice::from_raw_parts_mut,
};

use super::{Guid, Str16};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    util::interface,
};

pub mod raw;
use alloc::vec::Vec;
use raw::{
    RawBltOperation, RawBltPixel, RawGraphicsInfo, RawGraphicsOutput, RawPixelFormat,
    RawSimpleTextOutput, RawTextMode,
};

/// Text foreground attributes for [SimpleTextOutput]
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
// Uefi wants them as usize but SetAttributes wants u32 but actually its all one
// byte
pub struct TextForeground(usize);

impl TextForeground {
    pub const BLACK: Self = Self(0x00);
    pub const BLUE: Self = Self(0x01);
    pub const GREEN: Self = Self(0x02);
    pub const CYAN: Self = Self(0x03);
    pub const RED: Self = Self(0x04);
    pub const MAGENTA: Self = Self(0x05);
    pub const BROWN: Self = Self(0x06);
    pub const LIGHT_GRAY: Self = Self(0x07);

    pub const BRIGHT: Self = Self(0x08);
    pub const DARK_GRAY: Self = Self(0x08);

    pub const LIGHT_BLUE: Self = Self(0x09);
    pub const LIGHT_GREEN: Self = Self(0x0A);
    pub const LIGHT_CYAN: Self = Self(0x0B);
    pub const LIGHT_RED: Self = Self(0x0C);
    pub const LIGHT_MAGENTA: Self = Self(0x0D);
    pub const YELLOW: Self = Self(0x0E);
    pub const WHITE: Self = Self(0x0F);
}

/// Text background attributes for [SimpleTextOutput]
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
// Uefi wants them as usize but SetAttributes wants u32 but actually its all one
// byte??
pub struct TextBackground(usize);

impl TextBackground {
    pub const BLACK: Self = Self(0x00);
    pub const BLUE: Self = Self(0x01);
    pub const GREEN: Self = Self(0x02);
    pub const CYAN: Self = Self(0x03);
    pub const RED: Self = Self(0x04);
    pub const MAGENTA: Self = Self(0x05);
    pub const BROWN: Self = Self(0x06);
    pub const LIGHT_GRAY: Self = Self(0x07);
}

// interface!(SimpleTextInput(RawSimpleTextInput));

interface!(SimpleTextOutput(RawSimpleTextOutput));

impl<'table> SimpleTextOutput<'table> {
    pub fn output_string(&self, string: &str) -> Result<()> {
        let out = self.interface().output_string;
        let s: Vec<u16> = string.encode_utf16().chain(once(0)).collect();
        // Safety: s is a nul terminated string
        unsafe { out(self.interface, s.as_ptr()) }.into()
    }

    /// Use these colors for all output to this Protocol within the
    /// closure, restoring them afterwards.
    ///
    /// Note that this will affect ALL output to this protocol, from anywhere in
    /// the program.
    /// In particular, this includes usage of the log macros.
    pub fn with_attributes<F: FnMut()>(
        &self,
        fore: TextForeground,
        back: TextBackground,
        mut f: F,
    ) -> Result<()> {
        let cur = self.attributes();
        self.set_attributes(fore, back)?;
        f();
        self.set_attributes(cur.0, cur.1)
    }

    /// Use this foreground color for the duration of the `f` call.
    pub fn with_foreground<F: FnMut()>(&self, fore: TextForeground, f: F) -> Result<()> {
        self.with_attributes(fore, self.attributes().1, f)
    }

    /// Use this background color for the duration of the `f` call.
    pub fn with_background<F: FnMut()>(&self, back: TextBackground, f: F) -> Result<()> {
        self.with_attributes(self.attributes().0, back, f)
    }

    /// Set the text background color
    pub fn set_background(&self, back: TextBackground) -> Result<()> {
        self.set_attributes(self.attributes().0, back)
    }

    /// Current text attributes
    pub fn attributes(&self) -> (TextForeground, TextBackground) {
        // Safety: Construction ensures these are valid
        let mode = unsafe { *self.interface().mode };
        let attr = mode.attribute as u8;
        let f = attr & 0xF;
        let b = attr >> 4;

        (TextForeground(f.into()), TextBackground(b.into()))
    }

    pub fn set_attributes(&self, fore: TextForeground, back: TextBackground) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().set_attribute)(self.interface, fore.0 | back.0 << 4) }.into()
    }

    /// Reset the device associated with this protocol
    ///
    /// Clears the screen, resets cursor position.
    pub fn reset(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().reset)(self.interface, false) }.into()
    }

    /// Clears the screen, resets cursor position.
    pub fn clear(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().clear_screen)(self.interface) }.into()
    }

    /// Enables the cursor
    pub fn enable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().enable_cursor)(self.interface, true) }.into()
    }

    /// Disables the cursor
    pub fn disable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().enable_cursor)(self.interface, false) }.into()
    }

    /// Set the terminal mode to number `mode`
    pub fn set_mode(&self, mode: u32) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().set_mode)(self.interface, mode as usize) }.into()
    }

    /// Query terminal mode number `mode`
    ///
    /// # Note
    ///
    /// UEFI defines:
    ///
    /// - mode `0` as `80x25`
    /// - mode `1` as `80x80` If this doesn't exist
    /// - mode `2` and onwards as implementation specific
    pub fn query_mode(&self, mode: u32) -> Result<TextMode> {
        let mut cols = 0;
        let mut rows = 0;
        // Safety: Construction ensures these are valid
        let ret = unsafe {
            (self.interface().query_mode)(self.interface, mode as usize, &mut cols, &mut rows)
        };
        if ret.is_success() {
            let mode = TextMode::new(mode, (cols, rows));
            Ok(mode)
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Terminal output modes
    pub fn modes(&self) -> impl Iterator<Item = Result<TextMode>> + '_ {
        let mut mode = 0;
        core::iter::from_fn(move || {
            if mode >= self.max_mode() {
                return None;
            }
            let m = self.query_mode(mode as u32);
            mode += 1;
            Some(m)
        })
    }

    /// Current [`TextMode`]
    pub fn mode(&self) -> Result<TextMode> {
        // Safety: types
        let mode = unsafe { (*self.interface().mode).mode } as u32;
        let info = self.query_mode(mode)?;
        Ok(TextMode::new(mode, info.size()))
    }

    fn max_mode(&self) -> i32 {
        // Safety: Type system
        unsafe { (*self.interface().mode).max_mode }
    }
}

/// All failures are treated as [`EfiStatus::DEVICE_ERROR`].
///
/// Warnings are ignored. Ending Newlines are turned into \n\r.
/// Interior newlines are not, yet.
impl<'t> Write for SimpleTextOutput<'t> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let ret = match self.output_string(s) {
            Ok(()) => Ok(()),
            Err(e) if e.status().is_warning() => Ok(()),
            Err(_) => Err(fmt::Error),
        };
        if s.ends_with('\n') {
            ret.and_then(|_| self.output_string("\r").map_err(|_| fmt::Error))
        } else {
            ret
        }
    }
}

// TODO: Report clippy bug for GUID
#[allow(clippy::undocumented_unsafe_blocks)]
unsafe impl<'table> super::Protocol<'table> for SimpleTextOutput<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x38, 0x74, 0x77, 0xc2, 0x69, 0xc7, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69,
            0x72, 0x3b,
        ])
    };

    type Raw = RawSimpleTextOutput;

    unsafe fn from_raw(this: *mut RawSimpleTextOutput) -> Self {
        SimpleTextOutput::new(this)
    }
}

/// UEFI Text Mode Information
#[derive(Debug)]
pub struct TextMode {
    /// Mode number
    mode: u32,

    /// (Cols, Rows)
    size: (usize, usize),
}

impl TextMode {
    fn new(mode: u32, size: (usize, usize)) -> Self {
        Self { mode, size }
    }

    /// Mode number
    pub fn mode(&self) -> u32 {
        self.mode
    }

    /// Size (Cols, Rows)
    pub fn size(&self) -> (usize, usize) {
        self.size
    }
}

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
    /// `buffer` must be at least `width * height * size_of::<RawBltPixel>()`
    /// or else `INVALID_PARAMETER` will be returned.
    ///
    /// Buffer is BGR formatted 32-bit pixels
    pub fn blt(
        &self,
        buffer: &[u8],
        op: BltOperation,
        src: (usize, usize),
        dest: (usize, usize),
        res: (usize, usize),
        delta: usize,
    ) -> Result<()> {
        if buffer.len() < (res.0 * res.1 * size_of::<RawBltPixel>()) {
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
                delta,
            )
        }
        .into()
    }

    /// Get a mutable byte slice to the current framebuffer
    ///
    /// Note that each pixel `(x, y)`
    /// is at the `<size of a pixel> *`[`GraphicsMode::stride`]
    pub fn framebuffer(&self) -> Result<&mut [u8]> {
        // FIXME: This probably isnt sound?
        // Need some sort of token to prevent changing the framebuffer?
        // actually what about printing, that causes the gpu to modify it?
        // Volatile???
        // Safety:
        unsafe {
            let mode = &*self.interface().mode;

            let fb = {
                let ptr = mode.fb_base as *mut u8;
                let len = mode.fb_size;
                from_raw_parts_mut(ptr, len)
            };
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
