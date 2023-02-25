//! Tests that unexpected crates fail nicely
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(crate("bytes"))]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
