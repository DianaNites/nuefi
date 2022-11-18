//! Supported/known UEFI Protocols

use core::{
    fmt::{self, Write},
    marker::PhantomData,
    mem::MaybeUninit,
};

use crate::error::{EfiStatus, Result};

type Void = *mut [u8; 0];

#[derive(Debug)]
#[repr(C)]
pub struct SimpleTextInput {
    //
}

// #[derive(Debug)]
#[repr(C)]
pub struct RawSimpleTextOutput {
    reset: *mut Void, // EFI_TEXT_RESET,
    // output_string: *mut Void,       // EFI_TEXT_STRING,
    output_string: unsafe extern "efiapi" fn(this: *mut Self, string: *const u16) -> EfiStatus, /* EFI_TEXT_STRING, */
    test_string: *mut Void,         // EFI_TEXT_TEST_STRING,
    query_mode: *mut Void,          // EFI_TEXT_QUERY_MODE,
    set_mode: *mut Void,            // EFI_TEXT_SET_MODE,
    set_attribute: *mut Void,       // EFI_TEXT_SET_ATTRIBUTE,
    clear_screen: *mut Void,        // EFI_TEXT_CLEAR_SCREEN,
    set_cursor_position: *mut Void, // EFI_TEXT_SET_CURSOR_POSITION,
    enable_cursor: *mut Void,       // EFI_TEXT_ENABLE_CURSOR,
    mode: *mut Void,                // SIMPLE_TEXT_OUTPUT_MODE,
}

impl RawSimpleTextOutput {
    pub const GUID: [u8; 16] = [
        0x38, 0x74, 0x77, 0xc2, 0x69, 0xc7, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72,
        0x3b,
    ];
}

/// The UEFI Boot services
#[repr(transparent)]
pub struct SimpleTextOutput<'table> {
    /// Lifetime conceptually tied to [`crate::SystemTable`]
    interface: *mut RawSimpleTextOutput,
    phantom: PhantomData<&'table mut RawSimpleTextOutput>,
}

impl<'table> SimpleTextOutput<'table> {
    /// Create new BootServices
    ///
    /// # Safety
    ///
    /// - Must be valid pointer
    pub(crate) unsafe fn new(this: *mut RawSimpleTextOutput) -> Self {
        Self {
            interface: this,
            phantom: PhantomData,
        }
    }

    pub fn output_string(&self, string: &str) -> Result<()> {
        let out = unsafe { (*self.interface).output_string };
        let mut fin = EfiStatus::SUCCESS;
        // FIXME: Horribly inefficient
        for (i, char) in string.encode_utf16().enumerate() {
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
