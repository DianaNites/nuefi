use alloc::string::ToString;
use core::{arch::asm, fmt::Write, mem::size_of};

use log::{debug, info, trace};
use nuefi::{
    error::Status,
    proto::{
        loaded_image::{LoadedImage, LoadedImageDevicePath},
        Protocol,
    },
    table::raw::RawSystemTable,
    Boot,
    EfiHandle,
    SystemTable,
};
use raw_cpuid::CpuId;
use x86_64::registers::control::{Cr0, Cr0Flags};

use crate::{
    ensure,
    imp::{TestError, TestExt, TestExt2},
    TestResult,
};

mod basic;
pub use basic::basic_tests;

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
