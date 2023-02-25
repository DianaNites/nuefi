//! Test that the log feature works correctly
//!
//! as best we can considering the trybuild limitations
#![allow(unused_imports)]
use log::set_logger;
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(
    // Test that the full syntax works as documented
    // TODO: fail-test for `log()`? or pass test for it?
    log(color, all, exclude("", "")),
)]
fn e_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {

    // // If the macro worked correctly, the Logger should be set already
    // assert!(
    //     set_logger().is_err(),
    //     "Protocol macro didn't register logger?"
    // );
}
