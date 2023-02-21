//! This file tests when there are extra unexpected arguments
#[allow(unused_imports)]
use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>, extra: (), extra2: ()) -> Result<()> {
    Ok(())
}

fn main() {}
