use alloc::string::ToString;
use core::{arch::asm, fmt::Write, mem::size_of};

use nuefi::{
    error::Status,
    proto::loaded_image::LoadedImage,
    table::raw::RawSystemTable,
    Boot,
    EfiHandle,
    SystemTable,
};
use raw_cpuid::CpuId;
use x86_64::registers::control::{Cr0, Cr0Flags};

use crate::{ensure, imp::TestExt, TestResult};

pub fn test_2_70(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    let mut stdout = table.stdout();
    // let mut stdout = Stdout;
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
            "fnstcw word ptr [{}]",
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

    Ok(())
}

pub fn test_panic(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    panic!("Test panic");
    Ok(())
}

/// Test basic functionality required for the rest of the test environment to
/// run works as expected
///
/// Ideally, everything described here should be *exactly* what is required to
/// run the test suite.
///
/// To run the automated test infrastructure:
///
/// - A valid [`SystemTable`]
/// - A supported UEFI Revision
/// - [`SimpleTextOutput`] / `stdout`
///     - Actually WE don't..
/// - [`LoadedImage`]
/// - [`Scope`]
/// - [`DevicePath`]
/// - [`UefiString`]
pub fn basic_tests(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    let mut stdout = table.stdout();
    // let mut stdout = Stdout;

    let uefi_revision = table.uefi_revision();
    let boot = table.boot();

    let us = boot.open_protocol::<LoadedImage>(handle)?.missing()?;

    match (uefi_revision.major(), uefi_revision.minor()) {
        (2, x) if x >= 7 => {
            test_2_70(handle, table)?;
        }
        (y, x) => {
            writeln!(&mut stdout, "Unsupported UEFI revision {y}.{x}")?;
            return Err(Status::UNSUPPORTED.into());
        }
    }

    Ok(())
}
