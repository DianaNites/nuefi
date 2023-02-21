#[allow(unused_imports)]
use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry(crate = b"bytes", fake = true)]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
