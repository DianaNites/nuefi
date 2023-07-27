//! Raw UEFI data types

use crate::nuefi_core::base::{Char16, Status};

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

#[derive(Debug)]
#[repr(C)]
pub struct RawSimpleTextOutput {
    pub reset: Option<unsafe extern "efiapi" fn(this: *mut Self, extended: bool) -> Status>,

    pub output_string:
        Option<unsafe extern "efiapi" fn(this: *mut Self, string: *const Char16) -> Status>,

    pub test_string:
        Option<unsafe extern "efiapi" fn(this: *mut Self, string: *const Char16) -> Status>,

    pub query_mode: Option<
        unsafe extern "efiapi" fn(
            this: *mut Self,
            mode: usize,
            cols: *mut usize,
            rows: *mut usize,
        ) -> Status,
    >,

    pub set_mode: Option<unsafe extern "efiapi" fn(this: *mut Self, mode: usize) -> Status>,

    pub set_attribute: Option<unsafe extern "efiapi" fn(this: *mut Self, attr: usize) -> Status>,

    pub clear_screen: Option<unsafe extern "efiapi" fn(this: *mut Self) -> Status>,

    pub set_cursor_position:
        Option<unsafe extern "efiapi" fn(this: *mut Self, cols: usize, rows: usize) -> Status>,

    pub enable_cursor: Option<unsafe extern "efiapi" fn(this: *mut Self, visible: bool) -> Status>,
    pub mode: *mut RawTextMode,
}
