//! UEFI Console related protocols
use core::fmt::{self, Write};

use super::{Guid, Str16};
use crate::{
    error::{EfiStatus, Result},
    util::interface,
};

pub mod raw;
use raw::RawSimpleTextOutput;

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
        let mut fin = EfiStatus::SUCCESS;
        // FIXME: Horribly inefficient
        for (_i, char) in string.encode_utf16().enumerate() {
            // for (i, char) in string.chars().enumerate() {
            // let char = if char.len_utf16();
            let buf = [char, 0];
            // Safety: Buf is a nul terminated string
            let ret = unsafe { out(self.interface, buf.as_ptr()) };
            if ret.is_error() {
                return ret.into();
            }
            if ret.is_warning() {
                fin = ret;
            }
        }
        fin.into()
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
