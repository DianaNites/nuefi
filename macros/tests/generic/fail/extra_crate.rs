//! Make sure invalid extra values fail
use nuefi::{entry, error::Result, table::Boot, EfiHandle, SystemTable};

#[entry(crate("nuefi", "text", this::is::wrong))]
fn ee_main(_handle: EfiHandle, _table: SystemTable<Boot>) -> Result<()> {
    Ok(())
}

fn main() {}
