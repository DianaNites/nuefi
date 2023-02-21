#![allow(unused_imports, unused_variables, clippy::let_and_return, dead_code)]
#![warn(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]
#![no_std]
#![feature(alloc_error_handler)]
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
pub use macros::entry;
use table::{raw::RawSystemTable, Boot};

pub use crate::table::SystemTable;

pub mod error;
pub mod logger;
pub mod mem;
pub mod proto;
pub mod string;
pub mod table;
mod util;

/// Handle to the SystemTable. Uses Acquire/Release
static TABLE: AtomicPtr<RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to the images [`EfiHandle`]. Uses Relaxed, sync with [`TABLE`]
static HANDLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to something in UEFI firmware
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
///
/// # Example
///
/// ```rust
/// # use uefi as nuefi;
/// use nuefi as uefi2;
/// use uefi2::entry;
/// use uefi2::EfiHandle;
/// use uefi2::SystemTable;
/// use uefi2::table::Boot;
/// use uefi2::error::Result;
///
/// /// - Rename the crate internally to `uefi2`
/// /// - Enable some internal logging after startup/during exit
/// ///     - This uses the `log` crate, and works if you set up a logger
/// /// - `delay(n)` - Enable a 30 second delay if `e_main` returns `Err`, displaying the error for debugging.
/// /// - `panic` - Enables a default panic handler implementation
/// ///     - This implementation allows changing at runtime
/// /// - `alloc` - Enables a default alloc error handler implementation
/// ///     - This implementation allows changing at runtime
/// #[entry(crate = "uefi2", log, delay(30))]
/// fn e_main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
///     Ok(())
/// }
///
/// # fn main() {}
/// ```
#[no_mangle]
extern "efiapi" fn efi_main(image: EfiHandle, system_table: *mut RawSystemTable) -> EfiStatus {
    extern "Rust" {
        fn __internal__nuefi__main(
            handle: EfiHandle,
            table: SystemTable<Boot>,
        ) -> error::Result<()>;
        static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool>;
        static __INTERNAL_NUEFI_EXIT_DURATION: Option<u64>;
        static __INTERNAL_NUEFI_LOG: Option<bool>;
    }
    // Safety: Unsure how it can be unsafe tbh.
    let ext = unsafe { __INTERNAL_NUEFI_YOU_MUST_USE_MACRO };

    // Safety: Unsure how it can be unsafe tbh.
    let dur = unsafe { __INTERNAL_NUEFI_EXIT_DURATION };

    // Safety: Unsure how it can be unsafe tbh.
    let log = unsafe { __INTERNAL_NUEFI_LOG };

    let log = if let Some(log) = log {
        log
    } else {
        return EfiStatus::INVALID_PARAMETER;
    };
    if image.0.is_null() || system_table.is_null() || !matches!(ext, Some(false)) {
        return EfiStatus::INVALID_PARAMETER;
    }

    // SAFETY: Pointer is valid from firmware
    let valid = unsafe { RawSystemTable::validate(system_table) };
    if let Err(e) = valid {
        return e.status();
    }
    HANDLE.store(image.0, Ordering::Relaxed);
    TABLE.store(system_table, Ordering::Release);
    // Safety: Main must exist or won't link.
    // Signature is verified by `__INTERNAL_NUEFI_YOU_MUST_USE_MACRO` above
    //
    // `system_table` is non-null, we trust it from firmware.
    let ret = unsafe { __internal__nuefi__main(image, SystemTable::new(system_table)) };

    if log {
        info!("Returned from user main with status {ret:?}");
    }
    match ret {
        Ok(()) => EfiStatus::SUCCESS,
        Err(e) => {
            if let Some(table) = get_boot_table() {
                if log {
                    error!("UEFI User main exited with error: {}", e);
                }
                if let Some(dur) = dur {
                    if log {
                        error!("Waiting {dur} seconds");
                    }
                    let _ = table.boot().stall(Duration::from_secs(dur));
                }
                // TODO: Exit prompt
            }

            e.status()
        }
    }
}

#[doc(hidden)]
pub mod handlers;

#[cfg(test)]
mod tests {
    #![allow(unreachable_code, unused_mut)]
    use super::*;
    use crate::{entry, error::Result};

    // TODO: Write more library/infrastructure for writing a mock library
    // slash actual UEFI implementation in software to test against,
    // or even use in hardware. lol.

    #[entry(crate = "self")]
    pub fn mock_main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
        let stdout = table.stdout();
        stdout.reset()?;
        loop {}
        // stdout.set_background(TextBackground::BLACK)?;
        Ok(())
    }

    #[test]
    fn miri() -> Result<()> {
        // setup();
        let id = 69420;
        // Safety: yes
        let st = unsafe { RawSystemTable::mock() };
        let st = &st as *const _ as *mut _;
        let image = EfiHandle(&id as *const _ as *mut _);
        // info!("{st:?}");
        let ret = efi_main(image, st);
        // info!("{ret:?}");
        //
        panic!("{:#?}", ret);
        Ok(())
    }
}
