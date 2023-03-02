//! UEFI Console related protocols
use core::{
    fmt::{self, Write},
    mem::size_of,
    slice::from_raw_parts_mut,
};

use super::Str16;
use crate::{
    error::{EfiStatus, Result},
    string::UefiString,
    util::interface,
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

interface!(
    #[Protocol("387477C2-69C7-11D2-8E39-00A0C969723B", crate("crate"))]
    SimpleTextOutput(RawSimpleTextOutput)
);

impl<'table> SimpleTextOutput<'table> {
    #[track_caller]
    pub fn output_string(&self, string: &str) -> Result<()> {
        let out = self.interface().output_string.unwrap();
        let s = UefiString::new(string);
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
        unsafe { (self.interface().set_attribute.unwrap())(self.interface, fore.0 | back.0 << 4) }
            .into()
    }

    /// Reset the device associated with this protocol
    ///
    /// Clears the screen, resets cursor position.
    pub fn reset(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().reset.unwrap())(self.interface, false) }.into()
    }

    /// Clears the screen, resets cursor position.
    pub fn clear(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().clear_screen.unwrap())(self.interface) }.into()
    }

    /// Enables the cursor
    pub fn enable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().enable_cursor.unwrap())(self.interface, true) }.into()
    }

    /// Disables the cursor
    pub fn disable_cursor(&self) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().enable_cursor.unwrap())(self.interface, false) }.into()
    }

    /// Set the terminal mode to number `mode`
    pub fn set_mode(&self, mode: u32) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { (self.interface().set_mode.unwrap())(self.interface, mode as usize) }.into()
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
            (self.interface().query_mode.unwrap())(
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

/// All failures are treated as [`EfiStatus::DEVICE_ERROR`].
///
/// Warnings are ignored. Ending Newlines are turned into \n\r.
/// Interior newlines are not.
// #[cfg(no)]
impl<'t> Write for SimpleTextOutput<'t> {
    // Rust does not thread `track_caller` through here
    // #[track_caller]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Make sure to select the right trait so we dont blow the stack
        <&Self as Write>::write_str(&mut &*self, s)
    }
}

impl<'t> Write for &SimpleTextOutput<'t> {
    #[track_caller]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let ret = match self.output_string(s) {
            Ok(()) => Ok(()),
            Err(e) if e.status() == EfiStatus::WARN_UNKNOWN_GLYPH => Ok(()),
            Err(_) => Err(fmt::Error),
        };
        if s.ends_with('\n') {
            ret.and_then(|_| self.output_string("\r").map_err(|_| fmt::Error))
        } else {
            ret
        }
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
