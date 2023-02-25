//! Tests whether duplicate argument failures are nice
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(log, delay(69), log, delay(420), crate("nuefi"), crate("dup"))]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
