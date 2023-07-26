//! UEFI Console related protocols
use core::{
    fmt::{self, Write},
    mem::size_of,
    slice::from_raw_parts_mut,
};

use crate::{
    error::{Result, Status},
    nuefi_core::interface,
    string::{UefiStr, UefiString},
};

pub mod raw;

use raw::RawSimpleTextOutput;

use crate::Protocol;

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

// Note: This Protocol's methods can't use any logging infrastructure because
// this protocol is, itself, used by logging. It will infinitely recurse.
interface!(
    #[Protocol("387477C2-69C7-11D2-8E39-00A0C969723B")]
    SimpleTextOutput(RawSimpleTextOutput)
);

impl<'table> SimpleTextOutput<'table> {
    /// Output `string` to the system console at the current cursor location
    #[track_caller]
    pub fn output_string(&self, string: &UefiStr) -> Result<()> {
        let out = self.interface().output_string.ok_or(Status::UNSUPPORTED)?;

        // Safety: s is a nul terminated string
        unsafe { out(self.interface, string.as_ptr()) }.into()
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
        unsafe {
            (self.interface().set_attribute.ok_or(Status::UNSUPPORTED)?)(
                self.interface,
                fore.0 | back.0 << 4,
            )
        }
        .into()
    }

    /// Reset the device associated with this protocol
    ///
    /// Clears the screen, resets cursor position.
    pub fn reset(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().reset.ok_or(Status::UNSUPPORTED)?)(self.interface, false) }
            .into()
    }

    /// Clears the screen, resets cursor position.
    pub fn clear(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().clear_screen.ok_or(Status::UNSUPPORTED)?)(self.interface) }
            .into()
    }

    /// Enables the cursor
    pub fn enable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().enable_cursor.ok_or(Status::UNSUPPORTED)?)(self.interface, true)
        }
        .into()
    }

    /// Disables the cursor
    pub fn disable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().enable_cursor.ok_or(Status::UNSUPPORTED)?)(self.interface, false)
        }
        .into()
    }

    /// Set the terminal mode to number `mode`
    pub fn set_mode(&self, mode: u32) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().set_mode.ok_or(Status::UNSUPPORTED)?)(self.interface, mode as usize)
        }
        .into()
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
            (self.interface().query_mode.ok_or(Status::UNSUPPORTED)?)(
                self.interface,
                mode as usize,
                &mut cols,
                &mut rows,
            )
        };
        if ret.is_success() {
            let mode = TextMode::new(mode, (cols, rows));
            Ok(mode)
        } else {
            Err(ret.into())
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

// Internal
impl<'table> SimpleTextOutput<'table> {
    /// Write a Rust UTF-8 to this protocol, *without* allocating.
    ///
    /// Transparently converts any `\n` into `\r\n`.
    ///
    /// To bypass this conversion, use a [`UefiStr`]
    // Note: Qemu UEFI in mon:stdio incorrectly treats LF as CRLF,
    // whereas graphically it will be, correctly, mangled. UEFI mandates CR.
    fn write_str_impl(&self, s: &str) -> fmt::Result {
        for c in s.as_bytes() {
            let data = if *c == b'\n' {
                [b'\r' as u16, *c as u16, b'\0' as u16]
            } else {
                [*c as u16, b'\0' as u16, b'\0' as u16]
            };

            // Safety: data is always valid
            let u = unsafe { UefiStr::from_ptr_len(data.as_ptr().cast_mut(), data.len()) };

            let ret = match self.output_string(&u) {
                Ok(()) => Ok(()),
                Err(e) if e.status() == Status::WARN_UNKNOWN_GLYPH => Ok(()),
                Err(_) => Err(fmt::Error),
            };
            ret?;
        }
        Ok(())
    }
}

/// All failures are treated as [`Status::DEVICE_ERROR`].
///
/// [`Status::WARN_UNKNOWN_GLYPH`] is ignored.
///
/// All `\n` bytes are transparently converted to `\r\n`.
///
/// This is guaranteed to not allocate.
// #[cfg(no)]
impl<'t> Write for SimpleTextOutput<'t> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_impl(s)
    }
}

// TODO: Figure out how to link to previous impl
/// This is guaranteed to not allocate.
impl<'t> Write for &SimpleTextOutput<'t> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_impl(s)
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
