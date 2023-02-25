//! Test that the log syntax fails nicely
#![allow(unused_imports)]
use log::set_logger;
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(
    // Test duplicates of the same, but different forms
    log(color,  all, fake, faker(), fakest = "", exclude(""), exclude(""), exclude = ""),
    log(color,  all),
    log(color,),
    log, log,
    log(all, targets("")),
    log(targets(""), all),
)]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
