//! Test that everything works correctly, including the internal static and
//! basic features
use nuefi as NotNuefi;
use NotNuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(
    // Test that it can use our `NotNuefi` import
    crate("NotNuefi"),

    // Test that the basic syntax works as documented
    log,
)]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

// If the macro worked correctly, this static should exist
extern "Rust" {
    static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool>;
}

fn main() {
    let x = unsafe { __INTERNAL_NUEFI_YOU_MUST_USE_MACRO };
    assert!(
        matches!(x, Some(false)),
        "Protocol Macro incorrectly handled `__INTERNAL_NUEFI_YOU_MUST_USE_MACRO`. Value was {x:?}"
    );
}
