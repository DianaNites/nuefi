//! Tests that unexpected crates fail nicely
use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry(crate = "bytes")]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
