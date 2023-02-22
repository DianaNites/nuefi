//! Tests whether `crate` actually works
use nuefi as NotNuefi;
use NotNuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

mod imp {
    use super::*;

    #[entry(crate = "NotNuefi")]
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
