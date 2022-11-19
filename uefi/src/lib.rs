#![allow(unused_imports, unused_variables, clippy::let_and_return, dead_code)]
#![no_std]
#![feature(abi_efiapi, alloc_error_handler)]
use core::{
    arch::asm,
    ffi::c_void,
    fmt::{self, Write},
    panic::PanicInfo,
    sync::atomic::{AtomicPtr, Ordering},
};

use error::EfiStatus;
use table::Boot;
pub use table::SystemTable;

pub mod error;
pub mod proto;
pub mod table;
mod util;

/// Handle to the SystemTable. Uses Acquire/Release
static TABLE: AtomicPtr<table::RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to the images [`EfiHandle`]. Uses Relaxed, sync with [`TABLE`]
static HANDLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

struct UefiLogger {
    //
}

#[derive(Debug)]
#[repr(transparent)]
pub struct EfiHandle(*mut c_void);

pub type MainCheck = fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;

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
    match ret {
        Ok(_) => EfiStatus::SUCCESS,
        Err(e) => e.status(),
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let table = TABLE.load(Ordering::Acquire);
    if !table.is_null() {
        let handle_p = HANDLE.load(Ordering::Relaxed);
        let handle = EfiHandle(handle_p);
        // Safety:
        // Handle is not null
        // handle must be valid or else this code could not be running
        let table: SystemTable<table::Internal> = unsafe { SystemTable::new(table) };
        if let Some(table) = table.as_boot() {
            let mut stdout = table.stdout();
            let _ = writeln!(stdout, "{info}");
            let boot = table.boot();

            #[cfg(no)]
            #[cfg(not(debug_assertions))]
            {
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
    }
    // Uselessly loop if we cant do anything else.
    // The UEFI watchdog will kill us eventually.
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Couldn't allocate {} bytes", layout.size())
}
