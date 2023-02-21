use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

extern "Rust" {
    static __INTERNAL_PRIVATE_NUEFI_MACRO_SIG_VERIFIED: bool;
}

fn main() {
    let _x = unsafe { __INTERNAL_PRIVATE_NUEFI_MACRO_SIG_VERIFIED };
}
