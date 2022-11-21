#![allow(unused_imports, unused_variables, clippy::let_and_return, dead_code)]
#![no_std]
#![feature(abi_efiapi, alloc_error_handler)]
extern crate alloc;

use core::{
    ffi::c_void,
    fmt::Write,
    panic::PanicInfo,
    sync::atomic::{AtomicPtr, Ordering},
    time::Duration,
};

use error::EfiStatus;
use log::{error, info};
use table::Boot;

pub use crate::table::SystemTable;

pub mod error;
pub mod logger;
pub mod mem;
pub mod proto;
pub mod table;
mod util;

/// Handle to the SystemTable. Uses Acquire/Release
static TABLE: AtomicPtr<table::RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to the images [`EfiHandle`]. Uses Relaxed, sync with [`TABLE`]
static HANDLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to something in UEFI firmware
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct EfiHandle(*mut c_void);

pub type MainCheck = fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;

/// Get the global [`SystemTable<Boot>`], if available
fn get_boot_table() -> Option<SystemTable<Boot>> {
    let table = TABLE.load(Ordering::Acquire);
    if table.is_null() {
        return None;
    }
    // Safety:
    // - Table is not null
    // - Table must be valid or else this code could not be running
    let table: SystemTable<table::Internal> = unsafe { SystemTable::new(table) };
    table.as_boot()
}

/// UEFI Entry point
///
/// Uses a user-provided main function of type [`MainCheck`] as the library
/// entry-point
///
/// This does some basic initial setup, preparing the user entry point from the
/// UEFI one, validating tables, handling `main`s return value.
#[no_mangle]
extern "efiapi" fn efi_main(
    image: EfiHandle,
    system_table: *mut table::RawSystemTable,
) -> EfiStatus {
    extern "Rust" {
        fn main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;
    }
    if image.0.is_null() || system_table.is_null() {
        return EfiStatus::INVALID_PARAMETER;
    }
    // SAFETY: Pointer is valid from firmware
    let valid = unsafe { table::RawSystemTable::validate(system_table) };
    if let Err(e) = valid {
        return e.status();
    }
    HANDLE.store(image.0, Ordering::Relaxed);
    TABLE.store(system_table, Ordering::Release);
    // Safety: Main must exist or won't link.
    // FIXME: Could be wrong signature until derive macro is written.
    // After that, its out of scope.
    //
    // system_table is non-null, valid from firmware.
    let ret = unsafe { main(image, SystemTable::new(system_table)) };
    info!("Returned from user main with status {ret:?}");
    match ret {
        Ok(_) => EfiStatus::SUCCESS,
        Err(e) => {
            if let Some(table) = get_boot_table() {
                error!("UEFI User main exited with error: {}", e);
                // TODO: Make configurable in the macro entry point.
                let _ = table.boot().stall(Duration::from_secs(5));
            }

            e.status()
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(table) = get_boot_table() {
        let mut stdout = table.stdout();
        let _ = writeln!(stdout, "{info}");

        #[cfg(no)]
        #[cfg(not(debug_assertions))]
        {
            let handle_p = HANDLE.load(Ordering::Relaxed);
            let handle = EfiHandle(handle_p);
            let boot = table.boot();
            // Just in case?
            if !handle.0.is_null() {
                let _ = boot.exit(handle, EfiStatus::ABORTED);
            }
            let _ = writeln!(
                stdout,
                "Failed to abort on panic. Call to `BootServices::Exit` failed. Handle was {:p}",
                handle_p
            );
        }
    }
    // Uselessly loop if we cant do anything else.
    // The UEFI watchdog will kill us eventually.
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Couldn't allocate {} bytes", layout.size())
}
