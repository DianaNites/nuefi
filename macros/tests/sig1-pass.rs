//! Test that everything works correctly, including the internal static
use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

extern "Rust" {
    static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool>;
}

fn main() {
    let _x = unsafe { __INTERNAL_NUEFI_YOU_MUST_USE_MACRO };
}
