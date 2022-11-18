//! Supported/known UEFI Protocols

type Void = *mut [u8; 0];

#[derive(Debug)]
#[repr(C)]
pub struct SimpleTextInput {
    //
}

#[derive(Debug)]
#[repr(C)]
pub struct SimpleTextOutput {
    reset: *mut Void,               // EFI_TEXT_RESET,
    output_string: *mut Void,       // EFI_TEXT_STRING,
    test_string: *mut Void,         // EFI_TEXT_TEST_STRING,
    query_mode: *mut Void,          // EFI_TEXT_QUERY_MODE,
    set_mode: *mut Void,            // EFI_TEXT_SET_MODE,
    set_attribute: *mut Void,       // EFI_TEXT_SET_ATTRIBUTE,
    clear_screen: *mut Void,        // EFI_TEXT_CLEAR_SCREEN,
    set_cursor_position: *mut Void, // EFI_TEXT_SET_CURSOR_POSITION,
    enable_cursor: *mut Void,       // EFI_TEXT_ENABLE_CURSOR,
    mode: *mut Void,                // SIMPLE_TEXT_OUTPUT_MODE,
}

impl SimpleTextOutput {
    //
}

impl SimpleTextOutput {
    pub const GUID: [u8; 16] = [
        0x38, 0x74, 0x77, 0xc2, 0x69, 0xc7, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72,
        0x3b,
    ];
}
