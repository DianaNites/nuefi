//! Tests whether `crate` actually works
#![allow(unused_imports)]
use nuefi::{error::Result, table::Boot, EfiHandle, SystemTable};
use uefi as nuefi;

mod imp {
    use super::*;
    #[macros::entry(crate = "nuefi")]
    #[no_mangle]
    fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
        Ok(())
    }
}

fn main() {
    extern "Rust" {
        fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()>;
    }
}
