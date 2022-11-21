//! UEFI Console related protocols
use core::fmt::{self, Write};

use super::{Guid, Str16};
use crate::{
    error::{EfiStatus, Result},
    util::interface,
};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct RawSimpleTextInput {
    //
}

// interface!(SimpleTextInput(RawSimpleTextInput));

#[derive(Debug)]
#[repr(C)]
struct RawMode {
    max_mode: i32,
    mode: i32,
    attribute: i32,
    cursor_column: i32,
    cursor_row: i32,
    cursor_visible: bool,
}

// TODO: Report bug to upstream Rust that derive(Debug) doesn't work for efiapi
// #[derive(Debug)]
#[repr(C)]
pub(crate) struct RawSimpleTextOutput {
    reset: unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> EfiStatus,
    output_string: unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus,
    test_string: unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus,
    query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode: usize,
        cols: *mut usize,
        rows: *mut usize,
    ) -> EfiStatus,
    set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode: usize) -> EfiStatus,
    set_attribute: unsafe extern "efiapi" fn(this: *mut Self, attr: usize) -> EfiStatus,
    clear_screen: unsafe extern "efiapi" fn(this: *mut Self) -> EfiStatus,
    set_cursor_position:
        unsafe extern "efiapi" fn(this: *mut Self, cols: usize, rows: usize) -> EfiStatus,
    enable_cursor: unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> EfiStatus,
    mode: *mut RawMode,
}

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

    /// Reset the device associated with this protocol
    ///
    /// Clears the screen, resets cursor position.
    pub fn reset(&self) -> Result<()> {
        unsafe { (self.interface().reset)(self.interface, false) }.into()
    }

    /// Clears the screen, resets cursor position.
    pub fn clear(&self) -> Result<()> {
        unsafe { (self.interface().clear_screen)(self.interface) }.into()
    }

    /// Enables the cursor
    pub fn enable_cursor(&self) -> Result<()> {
        unsafe { (self.interface().enable_cursor)(self.interface, true) }.into()
    }

    /// Disables the cursor
    pub fn disable_cursor(&self) -> Result<()> {
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

unsafe impl<'table> super::Protocol<'table> for SimpleTextOutput<'table> {
    const GUID: Guid = unsafe {
        Guid::from_bytes([
            0x38, 0x74, 0x77, 0xc2, 0x69, 0xc7, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69,
            0x72, 0x3b,
        ])
    };

    unsafe fn from_raw(this: *mut u8) -> Self {
        unsafe { SimpleTextOutput::new(this as *mut RawSimpleTextOutput) }
    }
}
