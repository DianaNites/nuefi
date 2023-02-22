//! Raw UEFI data types

use core::{fmt, ptr::null_mut};

use super::Str16;
use crate::error::EfiStatus;

#[derive(Debug)]
#[repr(C)]
pub struct RawSimpleTextInput {
    //
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RawTextMode {
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
    pub reset: Option<unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> EfiStatus>,

    pub output_string:
        Option<unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus>,

    pub test_string: Option<unsafe extern "efiapi" fn(this: *mut Self, string: Str16) -> EfiStatus>,

    pub query_mode: Option<
        unsafe extern "efiapi" fn(
            this: *mut Self,
            mode: usize,
            cols: *mut usize,
            rows: *mut usize,
        ) -> EfiStatus,
    >,

    pub set_mode: Option<unsafe extern "efiapi" fn(this: *mut Self, mode: usize) -> EfiStatus>,

    pub set_attribute: Option<unsafe extern "efiapi" fn(this: *mut Self, attr: usize) -> EfiStatus>,

    pub clear_screen: Option<unsafe extern "efiapi" fn(this: *mut Self) -> EfiStatus>,

    pub set_cursor_position:
        Option<unsafe extern "efiapi" fn(this: *mut Self, cols: usize, rows: usize) -> EfiStatus>,

    pub enable_cursor:
        Option<unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> EfiStatus>,
    pub mode: *mut RawTextMode,
}

impl RawSimpleTextOutput {
    pub(crate) const fn mock() -> Self {
        unsafe extern "efiapi" fn reset(
            this: *mut RawSimpleTextOutput,
            extended: bool,
        ) -> EfiStatus {
            EfiStatus::SUCCESS
        }

        unsafe extern "efiapi" fn output_string(
            this: *mut RawSimpleTextOutput,
            string: Str16,
        ) -> EfiStatus {
            EfiStatus::SUCCESS
        }

        unsafe extern "efiapi" fn clear_screen(this: *mut RawSimpleTextOutput) -> EfiStatus {
            EfiStatus::SUCCESS
        }

        Self {
            reset: Some(reset),
            output_string: Some(output_string),
            test_string: None,
            query_mode: None,
            set_mode: None,
            set_attribute: None,
            clear_screen: Some(clear_screen),
            set_cursor_position: None,
            enable_cursor: None,
            mode: null_mut(),
        }
    }
}
