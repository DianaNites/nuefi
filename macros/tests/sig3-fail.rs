//! This file tests when there is a missing argument
use nuefi::{entry, error::Result, EfiHandle};

#[entry]
fn e_main(_handle: EfiHandle) -> Result<()> {
    Ok(())
}

fn main() {}
