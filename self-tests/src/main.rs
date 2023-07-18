#![no_std]
#![no_main]

use core::fmt::Write;

use nuefi::{
    entry,
    error::{Result, Status},
    Boot,
    EfiHandle,
    SystemTable,
};

#[entry(panic, alloc)]
fn main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
    let mut stdout = table.stdout();
    writeln!(&mut stdout, "Firmware Vendor {}", table.firmware_vendor())?;
    writeln!(
        &mut stdout,
        "Firmware Revision {}",
        table.firmware_revision()
    )?;
    writeln!(&mut stdout, "UEFI Revision {:?}", table.uefi_revision())?;

    Err(Status::UNSUPPORTED.into())
}
