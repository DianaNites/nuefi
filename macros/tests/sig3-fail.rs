//! This file tests when there is a missing argument
#[allow(unused_imports)]
use ::uefi::{error::Result, table::Boot, EfiHandle, SystemTable};

#[macros::entry]
fn e_main(_handle: EfiHandle) -> Result<()> {
    Ok(())
}

fn main() {}
