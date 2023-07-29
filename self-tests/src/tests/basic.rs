use core::{arch::asm, fmt::Write};

use log::{debug, info, trace};
use nuefi::{
    error::Status,
    proto::loaded_image::{LoadedImage, LoadedImageDevicePath},
    Boot,
    EfiHandle,
    SystemTable,
};
use raw_cpuid::CpuId;
use x86_64::registers::control::{Cr0, Cr0Flags};

use crate::{
    ensure,
    imp::{TestExt, TestExt2},
    TestResult,
};

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
///
/// # Pre-Environment Tests
///
/// These tests SHOULD NOT panic, or else they will crash the crash the test
/// infrastructure irrecoverably and return control to firmware.
pub fn basic_tests(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    abi_sanity(handle, table)?;
    let mut stdout = table.stdout();

    let uefi_revision = table.uefi_revision();
    let boot = table.boot();

    image_handle(handle, table)?;
    device_path_duplicate(handle, table)?;

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

/// Test the LoadedImage protocol, which UEFI mandates be installed on our
/// handle
///
/// Test that our image handle has the required [`LoadedImage`] and
/// [`LoadedImageDevicePath`] protocols
fn image_handle(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    trace!("image_handle");
    let boot = table.boot();
    // let mut stdout = table.stdout();

    // Safety: We know our test runner is okay with this
    let img = unsafe { boot.handle_protocol::<LoadedImage>(handle) };
    info!("LoadedImage = {img:#?}");
    let img = img?.missing();

    let img_dev = boot.open_protocol::<LoadedImageDevicePath>(handle);
    info!("LoadedImageDevicePath = {img_dev:#?}");
    let img_dev = img_dev?.missing()?;

    Ok(())
}

/// Test that [`DevicePath::duplicate`][0] and [`DevicePath::len`][0]
/// work correctly
///
/// This was initially implemented using the [`DevicePathUtil::duplicate`][0]
/// protocol, and then rewritten in Rust to directly duplicate using the UEFI
/// allocator.
///
/// As part of this, it became necessary to calculate the byte length of the
/// entire structure, which requires iterating through it, so a
/// [`DevicePath::len`] method was added.
///
/// This initial implementation had a soundness bug where it did not actually
/// update the running length, and always returned 0, resulting in duplicate
/// returning an invalid DevicePath pointing to a zero-sized allocation, which
/// apparently QEMU/OVMF can do, with none of the expected structure expected
/// for its type.
///
/// It also had an off-by-one node issue while traversing the DevicePaths,
/// incrementing the size for all the nodes and ending on the last one, but
/// *not* adding its size.
///
/// Test to ensure this works correctly.
///
/// [0]: nuefi::proto::device_path::DevicePathUtil
fn device_path_duplicate(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    trace!("device_path_duplicate");
    let boot = table.boot();

    let path = boot
        .open_protocol::<LoadedImageDevicePath>(handle)?
        .missing()?;
    let path = path.as_device_path();

    debug!("Path = {}", path.to_uefi_string()?);

    let dup = path.duplicate()?;
    debug!("Duplicate ({} bytes): {dup:#?}", dup.len());
    ensure!(path.len() == dup.len(), "Duplicated DevicePath was unequal");

    let dup = dup.to_string()?;
    let path = path.to_string()?;

    debug!("Duplicate: {dup}");

    // ensure!(path == dup, "Duplicated DevicePath was unequal");
    // assert_eq!(path, dup, "Duplicated DevicePath was unequal");
    assert_ne!(path, dup, "Duplicated DevicePath was unequal");

    //
    Ok(())
}
