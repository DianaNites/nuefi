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

static TABLE: AtomicPtr<table::RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

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
    TABLE.store(system_table, Ordering::Release);
    if true {
        panic!();
    }
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
        let table = unsafe { SystemTable::new(table) };
        let mut stdout = table.stdout();
        let _ = writeln!(stdout, "{info}");
        if let Some(boot) = table.boot() {
            //
        }
    }
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Couldn't allocate {} bytes", layout.size())
}
