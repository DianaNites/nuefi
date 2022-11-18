#![allow(unused_imports, unused_variables, clippy::let_and_return, dead_code)]
#![no_std]
#![feature(abi_efiapi, alloc_error_handler)]
use core::{
    ffi::c_void,
    fmt::{self, Write},
    panic::PanicInfo,
    sync::atomic::Ordering,
};

use error::EfiStatus;

pub mod error;
pub mod proto;
pub mod table;

#[derive(Debug)]
#[repr(transparent)]
pub struct EfiHandle(*mut c_void);

#[repr(transparent)]
pub struct SystemTable(*mut table::SystemTable);

#[repr(transparent)]
pub struct BootServices(*mut table::BootServices);

impl SystemTable {
    pub fn boot(&mut self) {
        // let x = &(*self.0);
        // unsafe { BootServices() }
        todo!()
    }
}

pub type MainCheck = fn(handle: EfiHandle, table: SystemTable) -> error::Result<()>;

#[no_mangle]
extern "efiapi" fn efi_main(image: EfiHandle, system_table: *mut table::SystemTable) -> EfiStatus {
    extern "Rust" {
        fn main(handle: EfiHandle, table: SystemTable) -> error::Result<()>;
    }
    if image.0.is_null() || system_table.is_null() {
        return EfiStatus::INVALID_PARAMETER;
    }
    // SAFETY: Pointer is valid from firmware
    let valid = unsafe { table::SystemTable::validate(system_table) };
    if let Err(e) = valid {
        return e.status();
    }
    // Safety: non-null, valid from firmware.
    // let table = unsafe { &*system_table };
    // Safety: Must exist or won't link.
    // FIXME: Could be wrong signature until derive macro is written.
    // After that, its out of scope.
    let ret = unsafe { main(image, SystemTable(system_table)) };
    match ret {
        Ok(_) => EfiStatus::SUCCESS,
        Err(e) => e.status(),
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Couldn't allocate {} bytes", layout.size())
}
