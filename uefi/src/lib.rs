#![allow(unused_imports, unused_variables)]
#![no_std]
#![feature(abi_efiapi)]
use core::{ffi::c_void, sync::atomic::Ordering};

#[repr(transparent)]
pub struct EfiHandle(*mut c_void);

pub type EfiStatus = usize;
pub type SystemTable = c_void;

#[no_mangle]
pub extern "efiapi" fn efi_main(image: EfiHandle, system_table: *mut SystemTable) -> EfiStatus {
    extern "Rust" {
        fn main() -> EfiStatus;
    }
    // Safety: Must exist or won't link.
    unsafe { main() }
}
