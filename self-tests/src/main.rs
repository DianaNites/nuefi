#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unreachable_code,
    clippy::no_effect
)]
#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::ToString;
use core::{arch::asm, fmt::Write, mem::size_of};

use nuefi::{
    entry,
    error::{Result, Status},
    table::raw::RawSystemTable,
    Boot,
    EfiHandle,
    SystemTable,
};
use qemu_exit::{QEMUExit, X86};
use raw_cpuid::CpuId;
use runs_inside_qemu::runs_inside_qemu;
use x86_64::registers::control::{Cr0, Cr0Flags};

// Only 127 codes are possible because linux.
const EXIT: X86 = X86::new(0x501, 69);

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // Safety: Yes
        unsafe { qemu_out(s.as_bytes()) };
        Ok(())
    }
}

macro_rules! ensure {
    ($stdout:expr, $e:expr $(, $m:expr)?) => {{
        write!($stdout, "Testing ")?;
        $(
            write!($stdout, "{}: ", $m)?;
        )?
        write!($stdout, "`{}` = ", stringify!($e))?;
        if !{ $e } {
            writeln!($stdout, "FAILED")?;
        } else {
            writeln!($stdout, "SUCCESS")?;
        }
    }};
}

fn test_2_70(handle: EfiHandle, table: SystemTable<Boot>, stdout: &mut Stdout) -> Result<()> {
    writeln!(stdout, "Starting testing of UEFI 2.7.0")?;

    let hdr = table.header();
    let uefi_revision = table.uefi_revision();

    ensure!(stdout, uefi_revision.major() == 2);
    ensure!(stdout, uefi_revision.minor() == 7);
    ensure!(stdout, uefi_revision.patch() == 0);
    ensure!(stdout, uefi_revision.to_string() == "2.7");
    ensure!(stdout, hdr.signature == RawSystemTable::SIGNATURE);
    ensure!(stdout, hdr.revision == RawSystemTable::REVISION_2_70);
    // actual `efi_main` should be validating these anyway
    ensure!(stdout, hdr.crc32 != 0);
    ensure!(stdout, hdr.reserved == 0);
    ensure!(stdout, hdr.size as usize == size_of::<RawSystemTable>());

    let cpuid = CpuId::new();
    let info = cpuid.get_feature_info().ok_or(Status::UNSUPPORTED)?;

    let mut word: u16 = 0;
    // Safety: loads a 16 bit value
    unsafe {
        asm!(
            "fnstcw [{}]",
            in(reg) &mut word,
            options(nostack),
        )
    };
    ensure!(stdout, word == 0x037F, "x87 FPU Control Word");

    let cr0 = Cr0::read();
    ensure!(
        stdout,
        !cr0.contains(Cr0Flags::EMULATE_COPROCESSOR | Cr0Flags::TASK_SWITCHED),
        "Task Switch and FP Emulation exceptions off"
    );

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
    writeln!(&mut stdout, "UEFI Revision {}", uefi_revision)?;

    writeln!(&mut stdout, "Successfully initialized testing core")?;

    match (uefi_revision.major(), uefi_revision.minor()) {
        (2, x) if x >= 7 => {
            test_2_70(handle, table, &mut stdout)?;
        }
        (y, x) => {
            writeln!(&mut stdout, "Unsupported UEFI revision {y}.{x}")?;
            if runs_inside_qemu().is_maybe_or_very_likely() {
                EXIT.exit_failure();
            }
            return Err(Status::UNSUPPORTED.into());
        }
    }

    if runs_inside_qemu().is_maybe_or_very_likely() {
        EXIT.exit_success();
    }

    Ok(())
}

/// # Safety
///
/// QEMU I/O port must be `0xE9` (the default)
#[inline]
unsafe fn qemu_out(b: &[u8]) {
    if runs_inside_qemu().is_definitely_not() {
        return;
    }
    for b in b {
        asm!(
            "out 0xE9, al",
            in("al") *b,
            options(
                nomem,
                preserves_flags,
                nostack,
            )
        );
    }
}

/// # Safety
///
/// QEMU exit I/O  must be `0x501` and 2 bytes (the default)
#[inline]
unsafe fn qemu_exit(x: u16) {
    if runs_inside_qemu().is_definitely_not() {
        return;
    }
    asm!(
        "mov dx, 0x501",
        "out dx, ax",
        in("ax") x,
        options(
            nomem,
            preserves_flags,
            nostack,
        )
    );
    qemu_out(b"Tried to exit qemu and failed\n");
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}
