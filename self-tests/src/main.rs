#![allow(unstable_name_collisions)]
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unreachable_code,
    clippy::no_effect,
    unused_mut
)]
#![no_std]
#![no_main]
extern crate alloc;

use alloc::{boxed::Box, string::ToString};
use core::{
    arch::asm,
    fmt::{self, write, Write},
    mem::size_of,
    ops::Deref,
};

use nuefi::{
    entry,
    error::{Result, Status, UefiError},
    proto::{
        console::SimpleTextOutput,
        loaded_image::{raw::RawLoadedImage, LoadedImage, LoadedImageDevicePath},
        media::LoadFile2,
        Protocol,
        Scope,
    },
    string::{Path, UefiString},
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

type TestFn = fn(EfiHandle, &SystemTable<Boot>) -> Result<()>;

type TestResult<T> = core::result::Result<T, TestError>;

// TODO: Figure out way to automatically register test functions
/// Test function and whether it "should fail" or not
static TESTS: &[(TestFn, bool)] = &[
    //
    (test_panic, true),
    (test_2_70, false),
];

#[derive(Debug, Clone, Copy)]
enum TestError {
    MissingProtocol(&'static str),
    Uefi(UefiError),
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestError::MissingProtocol(n) => write!(f, "missing protocol {n}"),
            TestError::Uefi(e) => write!(f, "{e}"),
        }
    }
}

impl From<UefiError> for TestError {
    fn from(value: UefiError) -> Self {
        TestError::Uefi(value)
    }
}

impl From<Status> for TestError {
    fn from(value: Status) -> Self {
        TestError::Uefi(value.into())
    }
}

trait TestExt
where
    Self: Sized,
{
    type OUT;

    fn missing(self) -> TestResult<Self::OUT>;
}

impl<'a, P> TestExt for Option<Scope<'a, P>>
where
    P: Protocol<'a>,
{
    type OUT = Scope<'a, P>;

    fn missing(self) -> TestResult<Self::OUT> {
        self.ok_or(TestError::MissingProtocol(P::NAME))
    }
}

#[derive(Debug, Clone, Copy)]
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

fn test_2_70(handle: EfiHandle, table: &SystemTable<Boot>) -> Result<()> {
    // let mut stdout = table.stdout();
    let mut stdout = Stdout;
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

fn test_panic(handle: EfiHandle, table: &SystemTable<Boot>) -> Result<()> {
    panic!("Test panic");
    Ok(())
}

/// Test basic functionality required for the rest of the test environment to
/// run
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
fn basic_tests(handle: EfiHandle, table: &SystemTable<Boot>) -> TestResult<()> {
    let mut stdout = table.stdout();
    // let mut stdout = Stdout;

    let boot = table.boot();

    let us = boot.open_protocol::<LoadedImage>(handle)?.missing()?;

    Ok(())
}

#[entry(panic, alloc)]
fn main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
    // let mut stdout = table.stdout();
    let mut stdout = Stdout;

    if let Err(e) = basic_tests(handle, &table) {
        writeln!(&mut stdout, "Error running Nuefi Test Suite: {e}")?;
        if runs_inside_qemu().is_maybe_or_very_likely() {
            EXIT.exit_failure();
        }
        return Err(Status::UNSUPPORTED.into());
    }

    // #[cfg(no)]
    {
        let boot = table.boot();

        let us = boot
            .open_protocol::<LoadedImage>(handle)?
            .ok_or(Status::UNSUPPORTED)?;

        let options = us.options().transpose()?;
        if let Some(options) = options {
            let idx = usize::from_le_bytes(options.try_into().map_err(|_| {
                let _ = writeln!(stdout, "Invalid load options");
                Status::INVALID_PARAMETER
            })?);
            writeln!(stdout, "Load Options: {idx}: {:#?}", options)?;

            if idx >= TESTS.len() {
                writeln!(stdout, "Invalid load options")?;
                return Err(Status::INVALID_PARAMETER.into());
            }
            TESTS[idx].0(handle, &table)?;

            return Ok(());
        }

        let file_dev = us.device().ok_or(Status::INVALID_PARAMETER)?;

        let file_path = boot
            .open_protocol::<LoadedImageDevicePath>(handle)?
            .ok_or(Status::UNSUPPORTED)?;
        let file_path = Path::new(file_path.as_device_path());

        writeln!(stdout, "Path = {}", file_path)?;
        writeln!(stdout, "Device = {:p}", file_dev)?;

        let dev = file_path.as_device();

        let max = TESTS.len();
        writeln!(stdout, "Running {} tests", max)?;
        for (idx, (test, fail)) in TESTS.iter().enumerate() {
            writeln!(stdout, "Running test {}/{}", idx + 1, max)?;
            let opt = idx.to_le_bytes();

            let img = boot.load_image_fs(handle, dev)?;

            // Scope has to end here or else we'll lock the protocol
            // for our child image, oops.
            {
                let load = boot
                    .open_protocol::<LoadedImage>(img)?
                    .ok_or(Status::INVALID_PARAMETER)?;
                // Safety: `opt` is valid until start_image below
                // FIXME: should have a safe API
                unsafe { load.set_options(&opt) };
            }

            // FIXME: No way to get ExitData
            // Safety: `img` is only run once, reinitialized each loop.
            let ret = unsafe { boot.start_image(img) };

            if ret.is_ok() || (ret.is_err() && *fail) {
                writeln!(stdout, "Test {}/{} completed successfully", idx + 1, max)?;
            } else {
                writeln!(stdout, "Test {}/{} completed unsuccessfully", idx + 1, max)?;
            }
        }
    }
    loop {}
    return Ok(());

    let fw_vendor = table.firmware_vendor();
    let fw_revision = table.firmware_revision();
    let uefi_revision = table.uefi_revision();

    writeln!(stdout, "Firmware Vendor {}", fw_vendor)?;
    writeln!(stdout, "Firmware Revision {}", fw_revision)?;
    writeln!(stdout, "UEFI Revision {}", uefi_revision)?;
    writeln!(stdout, "Successfully initialized testing core")?;

    match (uefi_revision.major(), uefi_revision.minor()) {
        (2, x) if x >= 7 => {
            test_2_70(handle, &table)?;
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

// TODO: Could run our own binary with different options to have isolated-ish
// testing?
// UEFI is identity mapped and privileged so we could,
// accidentally corrupt it, but.
//
// It would allow test functions,
// panicking test cases, and logging output.
//
// We can hook stdout/stderr, check return code, etc.
