//! Tests whether duplicate argument failures are nice
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(exit_prompt, log, exit_prompt, log, delay(69), delay(420), crate = "nuefi", crate = "nuefi")]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
