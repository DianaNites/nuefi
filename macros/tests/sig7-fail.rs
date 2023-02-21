//! Tests whether `crate` actually works
#![allow(unused_imports)]
use nuefi::{error::Result, table::Boot, EfiHandle, SystemTable};
use uefi as nuefi;

#[nuefi::entry(crate = "nuefi", exit_prompt, log, delay(69), delay(420))]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
