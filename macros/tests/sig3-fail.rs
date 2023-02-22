//! This file tests when there is a missing argument
use uefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry]
fn e_main(_handle: EfiHandle) -> Result<()> {
    Ok(())
}

fn main() {}
