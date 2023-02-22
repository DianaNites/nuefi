//! This file tests when there are extra unexpected arguments
use uefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>, extra: (), extra2: ()) -> Result<()> {
    Ok(())
}

fn main() {}
