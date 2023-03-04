//! Tests whether duplicate argument failures are nice
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(log, log, crate("nuefi"), crate("dup"))]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
