use alloc::string::ToString;
use core::{arch::asm, fmt::Write, mem::size_of};

use log::info;
use nuefi::{
    error::Status,
    proto::{loaded_image::LoadedImage, Protocol},
    table::raw::RawSystemTable,
    Boot,
    EfiHandle,
    SystemTable,
};
use raw_cpuid::CpuId;
use x86_64::registers::control::{Cr0, Cr0Flags};

use crate::{
    ensure,
    imp::{TestError, TestExt},
    TestResult,
};

pub fn test_2_70(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    let mut stdout = table.stdout();
    // let mut stdout = Stdout;
    writeln!(stdout, "Starting testing of UEFI 2.7.0")?;

    let hdr = table.header();
    let uefi_revision = table.uefi_revision();

    ensure!(uefi_revision.major() == 2);
    ensure!(uefi_revision.minor() == 7);
    ensure!(uefi_revision.patch() == 0);
    ensure!(uefi_revision.to_string() == "2.7");
    ensure!(hdr.signature == RawSystemTable::SIGNATURE);
    ensure!(hdr.revision == RawSystemTable::REVISION_2_70);
    // actual `efi_main` should be validating these anyway
    ensure!(hdr.crc32 != 0);
    ensure!(hdr.reserved == 0);
    ensure!(hdr.size as usize == size_of::<RawSystemTable>());

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
    abi_sanity(handle, table)?;
    let mut stdout = table.stdout();

    let uefi_revision = table.uefi_revision();
    let boot = table.boot();

    // Safety: We know our test runner is okay with this
    let us = unsafe {
        boot.handle_protocol::<LoadedImage>(handle)?
            .ok_or(TestError::MissingProtocol(LoadedImage::NAME))?
    };

    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn abi_sanity(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    info!("Testing our sanity");

    {
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
        ensure!(word == 0x037F, "x87 FPU Control Word");

        let cr0 = Cr0::read();
        ensure!(
            !cr0.contains(Cr0Flags::EMULATE_COPROCESSOR | Cr0Flags::TASK_SWITCHED),
            "Task Switch and FP Emulation exceptions off"
        );
    }

    // TODO: Figure out how to check ABI is correctly followed?
    // Is that even useful? Wildly out of scope?

    info!("Our sanity seems to have held, for now");

    Ok(())
}
