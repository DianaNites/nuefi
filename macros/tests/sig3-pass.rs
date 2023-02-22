//! Tests whether `crate` actually works
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};
use uefi as nuefi;

#[entry(crate = "nuefi", exit_prompt, log, delay(69))]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
