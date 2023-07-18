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

fn test_2_70(handle: EfiHandle, table: SystemTable<Boot>, stdout: &mut Stdout) -> Result<()> {
    writeln!(stdout, "Starting testing of UEFI 2.70")?;
    //
    Ok(())
}

#[entry(panic, alloc)]
fn main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
    // let mut stdout = table.stdout();
    let mut stdout = Stdout;

    let fw_vendor = table.firmware_vendor();
    let fw_revision = table.firmware_revision();
    let uefi_revision = table.uefi_revision();

    writeln!(&mut stdout, "Firmware Vendor {}", fw_vendor)?;
    writeln!(&mut stdout, "Firmware Revision {}", fw_revision)?;
    writeln!(&mut stdout, "UEFI Revision {:?}", uefi_revision)?;

    writeln!(&mut stdout, "Successfully initialized testing core")?;

    match uefi_revision {
        (2, x) if x >= 70 => {
            test_2_70(handle, table, &mut stdout)?;
        }
        (y, x) => {
            writeln!(&mut stdout, "Unsupported UEFI revision {y}.{x}")?;
            return Err(Status::UNSUPPORTED.into());
        }
    }

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
