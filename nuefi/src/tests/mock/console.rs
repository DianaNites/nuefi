use alloc::boxed::Box;
use core::ptr::null_mut;

use nuefi_core::{base::Char16, error::Status};

use crate::{proto::console::raw::RawSimpleTextOutput, string::UcsString};

#[derive(Debug)]
#[repr(C)]
pub struct MockConsole {
    this: RawSimpleTextOutput,

    /// Simple linear framebuffer
    screen: Box<[u16; 80 * 25]>,
}

impl MockConsole {
    pub fn new() -> Self {
        Self {
            this: RawSimpleTextOutput {
                reset: Some(Self::reset),
                output_string: Some(Self::output_string),
                test_string: None,
                query_mode: None,
                set_mode: None,
                set_attribute: None,
                clear_screen: Some(Self::clear_screen),
                set_cursor_position: None,
                enable_cursor: None,
                mode: null_mut(),
            },
            screen: Box::new([0u16; 80 * 25]),
        }
    }
}

impl MockConsole {
    unsafe extern "efiapi" fn reset(this: *mut RawSimpleTextOutput, _extended: bool) -> Status {
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn output_string(
        this: *mut RawSimpleTextOutput,
        string: *const Char16,
    ) -> Status {
        let this = &mut *(this as *mut Self);

        let s = UcsString::from_ptr(string);
        let len = s.as_slice().len();

        this.screen[..len].copy_from_slice(s.as_slice());

        Status::SUCCESS
    }

    unsafe extern "efiapi" fn clear_screen(this: *mut RawSimpleTextOutput) -> Status {
        let this = &mut *(this as *mut Self);
        this.screen.fill(0);
        Status::SUCCESS
    }
}

impl MockConsole {
    pub unsafe fn free(this: *const u8) {
        let this = this as *const Self;

        // Safety: Caller
        core::ptr::drop_in_place(this.cast_mut());
    }
}
