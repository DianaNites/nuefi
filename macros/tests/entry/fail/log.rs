//! Test that the log syntax fails nicely
#![allow(unused_imports)]
use log::set_logger;
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(
    // Test duplicates of the same, but different forms
    log(color,  all,),
    log,
)]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
