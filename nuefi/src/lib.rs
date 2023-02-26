//! A safe Rust UEFI library that provides an environment to safely
//! write applications and interact with firmware.
//!
//! # Quick Start
//!
//! Minimal quick start example, this will setup a UEFI entry point for you that
//! prints "Hello, world!".
//!
//! This example does not compile because of limitations in rustdoc.
//!
//! ```rust,compile_fail
//! // No standard library
//! #![no_std]
//!
//! // No main function
//! #![no_main]
//! use nuefi::{entry, EfiHandle, SystemTable, Boot, error};
//! use core::fmt::Write;
//!
//! // Generate the UEFI entry point
//! #[entry(
//!     // Generates a panic handler implementation for you!
//!     panic,
//!
//!     // Generates a global allocator for you!
//!     alloc,
//! )]
//! fn main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
//!     let mut stdout = table.stdout();
//!     writeln!(&mut stdout, "Hello, world!")?;
//!     Ok(())
//! }
//! ```
#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unreachable_code,
    clippy::let_and_return,
    clippy::diverging_sub_expression
)]
#![warn(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]
#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;

use core::{
    ffi::c_void,
    fmt::Write,
    panic::PanicInfo,
    ptr::addr_of,
    sync::atomic::{AtomicPtr, Ordering},
    time::Duration,
};

use error::EfiStatus;
use log::{error, info};
pub use macros::{entry, Protocol};
use table::raw::RawSystemTable;

pub use crate::table::{Boot, SystemTable};

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
/// #[entry(crate("uefi2"), log, delay(30))]
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
    }
    #[cfg(miri)]
    let (ext, dur) = {
        (
            Some(false), //
            Some(30),    //
        )
    };

    #[cfg(not(miri))]
    // Safety: Unsure how it can be unsafe tbh.
    let (ext, dur) = unsafe {
        if addr_of!(__INTERNAL_NUEFI_YOU_MUST_USE_MACRO).is_null()
            || addr_of!(__INTERNAL_NUEFI_EXIT_DURATION).is_null()
        {
            return EfiStatus::INVALID_PARAMETER;
        }
        (
            __INTERNAL_NUEFI_YOU_MUST_USE_MACRO,
            __INTERNAL_NUEFI_EXIT_DURATION,
        )
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

    info!("Returned from user main with status {ret:?}");
    match ret {
        Ok(()) => EfiStatus::SUCCESS,
        Err(e) => {
            if let Some(table) = get_boot_table() {
                error!("UEFI User main exited with error: {}", e);
                if let Some(dur) = dur {
                    error!("Waiting {dur} seconds");
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
    use alloc::{boxed::Box, vec::Vec};
    use core::mem::forget;

    use mock::{mock, MOCK_VENDOR};

    use super::*;
    use crate::{entry, error::Result, proto::graphics::GraphicsOutput};

    mod mock {
        use alloc::{boxed::Box, vec, vec::Vec};
        use core::{
            any::Any,
            mem::size_of,
            ptr::{addr_of, addr_of_mut, null_mut},
        };

        use crate::{
            error::EfiStatus,
            proto::{
                self,
                console::raw::RawSimpleTextOutput,
                graphics::{raw::RawGraphicsOutput, GraphicsOutput},
                Protocol,
                Str16,
            },
            table::raw::{
                Header,
                RawBootServices,
                RawRuntimeServices,
                RawSystemTable,
                Revision,
                CRC,
            },
            EfiHandle,
        };

        const MOCK_REVISION: Revision = Revision::new(2, 70);
        const MOCK_FW_REVISION: u32 = 69420;
        pub const MOCK_VENDOR: &str = "Mock Vendor";

        const fn mock_boot() -> RawBootServices {
            const MOCK_HEADER: Header = Header {
                signature: RawBootServices::SIGNATURE,
                revision: MOCK_REVISION,
                size: size_of::<RawBootServices>() as u32,
                crc32: 0,
                reserved: 0,
            };
            let b = [0u8; size_of::<RawBootServices>()];
            // Safety:
            // - All fields of `RawBootServices` are safely nullable/zero
            let mut t: RawBootServices = unsafe { core::mem::transmute::<_, _>(b) };
            t.header = MOCK_HEADER;
            t
        }

        const fn mock_run() -> RawRuntimeServices {
            const MOCK_HEADER: Header = Header {
                signature: RawRuntimeServices::SIGNATURE,
                revision: MOCK_REVISION,
                size: size_of::<RawRuntimeServices>() as u32,
                crc32: 0,
                reserved: 0,
            };
            let b = [0u8; size_of::<RawRuntimeServices>()];
            // Safety:
            // - All fields of `RawRuntimeServices` are safely nullable/zero
            let mut t: RawRuntimeServices = unsafe { core::mem::transmute::<_, _>(b) };
            t.header = MOCK_HEADER;
            t
        }

        const fn mock_out() -> RawSimpleTextOutput {
            unsafe extern "efiapi" fn reset(
                this: *mut RawSimpleTextOutput,
                extended: bool,
            ) -> EfiStatus {
                EfiStatus::SUCCESS
            }

            unsafe extern "efiapi" fn output_string(
                this: *mut RawSimpleTextOutput,
                string: Str16,
            ) -> EfiStatus {
                EfiStatus::SUCCESS
            }

            unsafe extern "efiapi" fn clear_screen(this: *mut RawSimpleTextOutput) -> EfiStatus {
                EfiStatus::SUCCESS
            }

            RawSimpleTextOutput {
                reset: Some(reset),
                output_string: Some(output_string),
                test_string: None,
                query_mode: None,
                set_mode: None,
                set_attribute: None,
                clear_screen: Some(clear_screen),
                set_cursor_position: None,
                enable_cursor: None,
                mode: null_mut(),
            }
        }

        const fn mock_gop() -> RawGraphicsOutput {
            unsafe extern "efiapi" fn set_mode(
                this: *mut RawGraphicsOutput,
                mode: u32,
            ) -> EfiStatus {
                EfiStatus::DEVICE_ERROR
            }

            RawGraphicsOutput {
                query_mode: None,
                set_mode: Some(set_mode),
                blt: None,
                mode: null_mut(),
            }
        }

        const fn mock_system() -> RawSystemTable {
            const MOCK_HEADER: Header = Header {
                signature: RawSystemTable::SIGNATURE,
                revision: MOCK_REVISION,
                size: size_of::<RawSystemTable>() as u32,
                crc32: 0,
                reserved: 0,
            };
            RawSystemTable {
                header: MOCK_HEADER,
                firmware_vendor: null_mut(),
                firmware_revision: MOCK_FW_REVISION,
                console_in_handle: EfiHandle(null_mut()),
                con_in: null_mut(),
                console_out_handle: EfiHandle(null_mut()),
                con_out: null_mut(),
                console_err_handle: EfiHandle(null_mut()),
                con_err: null_mut(),
                runtime_services: null_mut(),
                boot_services: null_mut(),
                number_of_table_entries: 0,
                configuration_table: null_mut(),
                _pad1: [0u8; 4],
            }
        }

        /// # Safety:
        ///
        /// `T` must not have uninit padding.
        const unsafe fn to_bytes<T>(this: &T) -> &[u8] {
            // Safety: `this` is valid by definition
            // Lifetime is bound to `this`
            unsafe { core::slice::from_raw_parts(this as *const T as *const u8, size_of::<T>()) }
        }

        pub fn mock() -> (Box<RawSystemTable>, Vec<Box<dyn Any>>) {
            let mut vendor = MOCK_VENDOR
                .encode_utf16()
                .chain([0])
                .collect::<Vec<u16>>()
                .into_boxed_slice();
            let mut system = Box::new(mock_system());
            let mut boot = Box::new(mock_boot());
            let mut run = Box::new(mock_run());
            let mut out = Box::new(mock_out());

            boot.locate_protocol = Some(locate_protocol);

            boot.header.crc32 = {
                let mut digest = CRC.digest();
                // Safety: We ensure in the definition that there is no uninit padding.
                unsafe { digest.update(to_bytes(&*boot)) };
                digest.finalize()
            };

            run.header.crc32 = {
                let mut digest = CRC.digest();
                // Safety: We ensure in the definition that there is no uninit padding.
                unsafe { digest.update(to_bytes(&*run)) };
                digest.finalize()
            };

            system.boot_services = addr_of_mut!(*boot);
            system.runtime_services = addr_of_mut!(*run);
            system.con_out = addr_of_mut!(*out);
            system.firmware_vendor = addr_of!(vendor[0]);

            system.header.crc32 = {
                let mut digest = CRC.digest();
                // Safety: We ensure in the definition that there is no uninit padding.
                unsafe { digest.update(to_bytes(&*system)) };
                digest.finalize()
            };

            (
                system,
                vec![
                    Box::new(boot),
                    Box::new(out),
                    Box::new(run),
                    Box::new(vendor),
                ],
            )
        }

        use imps::*;
        mod imps {
            use super::*;

            pub static mut MOCK_GOP: RawGraphicsOutput = mock_gop();

            pub unsafe extern "efiapi" fn locate_protocol(
                guid: *mut proto::Guid,
                key: *mut u8,
                out: *mut *mut u8,
            ) -> EfiStatus {
                let guid = *guid;
                if guid == GraphicsOutput::GUID {
                    out.write(addr_of_mut!(MOCK_GOP) as *mut _);
                    EfiStatus::SUCCESS
                } else {
                    out.write(null_mut());
                    EfiStatus::NOT_FOUND
                }
            }
        }
    }

    #[entry(crate("self"))]
    pub fn mock_main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
        let stdout = table.stdout();
        stdout.reset()?;
        let vendor = table.firmware_vendor();

        let boot = table.boot();

        let gop = boot.locate_protocol::<GraphicsOutput>()?.unwrap();
        let gop2 = boot.locate_protocol::<GraphicsOutput>()?.unwrap();
        let _ = gop.set_mode(69);
        info!("{:?}", gop2.mode());
        let _ = gop.set_mode(420);
        // panic!("{gop:?}");
        Ok(())
    }

    const IMAGE: EfiHandle = EfiHandle(69420 as *mut _);

    #[test]
    fn miri() -> Result<()> {
        // setup();
        let (mut st, _box) = { mock() };
        {
            let st = (&mut *st) as *mut RawSystemTable;
            // info!("{st:?}");
            let ret = efi_main(IMAGE, st);
            // info!("{ret:?}");

            if !ret.is_success() {
                panic!("{:#?}", ret);
            }

            // drop(_box);
            // Miri stack borrows complains because when header validation
            // makes the byte slice, for some reason that invalidates
            // `Box` from dropping itself. This has got to be a bug.
            //
            // TODO: Try and come up with a minimal repro.
            // Might be this bug?
            // <https://github.com/rust-lang/miri/issues/2728>
            forget(_box);
        }
        Ok(())
    }
}
