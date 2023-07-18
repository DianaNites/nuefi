#![allow(dead_code, unused_imports, unused_variables, unreachable_code)]
#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write};

use nuefi::{
    entry,
    error::{Result, Status},
    Boot,
    EfiHandle,
    SystemTable,
};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Safety: Yes
        unsafe { qemu_out(s.as_bytes()) };
        Ok(())
    }
}

#[entry(panic, alloc)]
fn main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
    // let mut stdout = table.stdout();
    let mut stdout = Stdout;
    writeln!(&mut stdout, "Firmware Vendor {}", table.firmware_vendor())?;
    writeln!(
        &mut stdout,
        "Firmware Revision {}",
        table.firmware_revision()
    )?;
    writeln!(&mut stdout, "UEFI Revision {:?}", table.uefi_revision())?;
    loop {
        unsafe {
            asm!("hlt");
        }
    }

    Err(Status::UNSUPPORTED.into())
}

/// # Safety
///
/// See [`out_byte`]
#[inline]
unsafe fn qemu_out(b: &[u8]) {
    for b in b {
        out_byte(*b);
    }
}

/// # Safety
///
/// QEMU I/O port must be `0xE9` (the default)
#[inline]
unsafe fn out_byte(b: u8) {
    core::arch::asm!(
        "out 0xE9, al",
        in("al") b,
        options(
            nomem,
            preserves_flags,
            nostack,
        )
    );
}
