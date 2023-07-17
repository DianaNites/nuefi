//! A safe Rust UEFI library that provides an environment to safely
//! write applications and interact with firmware.
//!
//! This library is designed to be easy to use more than it is a direct mapping
//! to UEFI firmware, though it does intend to support such use in practice
//! and in documentation, by documenting what actions, if any, are performed
//! "behind the scenes", and when.
//!
//! # Quick Start
//!
//! Minimal quick start example, this will setup a UEFI entry point for you that
//! prints "Hello, world!".
//!
//! This example does not compile because of limitations in rustdoc.
//!
//! ```rust,compile_fail
//! // We need both these options for a UEFI application
//! #![no_std]
//! #![no_main]
//!
//! // And these imports
//! use nuefi::{entry, EfiHandle, SystemTable, Boot, error};
//! use core::fmt::Write;
//!
//! // Generate the UEFI entry point
//! #[entry(
//!     // Generates a panic handler implementation for you
//!     panic,
//!
//!     // Generates a global allocator for you
//!     alloc,
//! )]
//! fn main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
//!     let mut stdout = table.stdout();
//!     writeln!(&mut stdout, "Hello, world!")?;
//!     Ok(())
//! }
//! ```
//!
//! # Environment
//!
//! UEFI on x86_64 is Uniprocessor, 64-bit, with 1:1 paging according to
//! the memory map, and no interrupts except for one timer.
//!
//! Note that it is legal for the user to, unsafely, change paging,
//! per [the spec][alt_page].
//! The application is required to restore the expected paging before using any
//! UEFI services, though.
//!
//! [alt_page]: <https://uefi.org/specs/UEFI/2.10/02_Overview.html#enabling-paging-or-alternate-translations-in-an-application>
#![allow(
    unused_imports,
    unused_variables,
    unused_mut,
    dead_code,
    unreachable_code,
    unused_unsafe,
    clippy::let_and_return,
    clippy::diverging_sub_expression,
    clippy::let_unit_value,
    clippy::never_loop
)]
// Enable these when actively working on them
// Empty or poor documentation blocks are worse than none at all,
// now they're harder to find.
// #![warn(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]
#![no_std]
// #![feature(alloc_error_handler)]
extern crate alloc;

extern crate self as nuefi;

use core::{
    ffi::c_void,
    fmt::Write,
    panic::PanicInfo,
    ptr::addr_of,
    sync::atomic::{AtomicPtr, Ordering},
    time::Duration,
};

use log::{error, info};
pub use macros::{entry, Protocol, GUID};
pub use nuefi_core::error;
use table::raw::RawSystemTable;

use crate::nuefi_core::base::Status;
pub use crate::table::{Boot, SystemTable};
pub mod logger;
pub mod mem;
pub mod proto;
pub mod string;
pub mod table;
mod util;

/// UEFI Core types
pub use nuefi_core;

/// Handle to the SystemTable. Uses Acquire/Release
static TABLE: AtomicPtr<RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to the images [`EfiHandle`]. Uses Relaxed, sync with [`TABLE`]
static HANDLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

pub use nuefi_core::base::Handle as EfiHandle;

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

/// Get the global Image [`EfiHandle`], if available
fn get_image_handle() -> Option<EfiHandle> {
    let _table = TABLE.load(Ordering::Acquire);
    let handle_p = HANDLE.load(Ordering::Relaxed);
    if !handle_p.is_null() {
        // Safety: `handle_p` was set in `efi_main` to our handle
        unsafe { Some(EfiHandle::new(handle_p)) }
    } else {
        None
    }
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
/// /// - `panic` - Enables a default panic handler implementation
/// ///     - This implementation allows changing at runtime
/// /// - `alloc` - Enables a default alloc error handler implementation
/// ///     - This implementation allows changing at runtime
/// #[entry(crate("uefi2"), log)]
/// fn e_main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
///     Ok(())
/// }
///
/// # fn main() {}
/// ```
// # Safety: UEFI Guarantees these are valid, and is the only one capable of doing so
// This is *the* UEFI entry point, and the only supported way to use this library.
#[no_mangle]
extern "efiapi" fn efi_main(image: EfiHandle, system_table: *mut RawSystemTable) -> Status {
    extern "Rust" {
        fn __internal__nuefi__main(
            handle: EfiHandle,
            table: SystemTable<Boot>,
        ) -> error::Result<()>;
        static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool>;
    }

    // Miri cannot yet read extern statics like ours
    #[cfg(miri)]
    let (ext,) = { (Some(false),) };

    #[cfg(not(miri))]
    // Safety: Unsure how it can be unsafe tbh.
    let (ext,) = unsafe {
        if addr_of!(__INTERNAL_NUEFI_YOU_MUST_USE_MACRO).is_null() {
            return Status::INVALID_PARAMETER;
        }
        (__INTERNAL_NUEFI_YOU_MUST_USE_MACRO,)
    };

    if image.as_ptr().is_null() || system_table.is_null() || !matches!(ext, Some(false)) {
        return Status::INVALID_PARAMETER;
    }

    // Safety:
    // - Assured pointer wasn't null above
    // - Firmware assures us this is a fully valid system table
    let valid = unsafe { RawSystemTable::validate(system_table) };
    if let Err(e) = valid {
        return e.status();
    }

    // Store a copy of the pointer to the image handle and system table
    HANDLE.store(image.as_ptr(), Ordering::Relaxed);
    TABLE.store(system_table, Ordering::Release);

    // Safety:
    // - Must exist or won't link
    // - Signature was verified by proc macro based on the existence of
    //   `__INTERNAL_NUEFI_YOU_MUST_USE_MACRO`
    //
    // - `system_table` was validated earlier
    let ret = unsafe { __internal__nuefi__main(image, SystemTable::new(system_table)) };
    match ret {
        Ok(()) => Status::SUCCESS,
        Err(e) => e.status(),
    }
}

#[doc(hidden)]
pub mod handlers;

#[cfg(test)]
mod tests {
    #![allow(unreachable_code, unused_mut)]
    use alloc::{boxed::Box, vec::Vec};
    use core::mem::{forget, size_of};

    use mock::{mock, MOCK_VENDOR};
    use nuefi_core::table::{Header, CRC};

    use super::*;
    use crate::{
        entry,
        error::{Result, Status},
        proto::{graphics::GraphicsOutput, loaded_image::LoadedImage},
    };

    mod mock {
        use alloc::{boxed::Box, vec, vec::Vec};
        use core::{
            any::Any,
            mem::size_of,
            ptr::{addr_of, addr_of_mut, null_mut},
        };

        use nuefi_core::{
            base::Char16,
            table::{Header, CRC},
        };

        use crate::{
            error::Status,
            proto::{
                self,
                console::raw::RawSimpleTextOutput,
                graphics::{raw::RawGraphicsOutput, GraphicsOutput},
                Protocol,
            },
            table::raw::{RawBootServices, RawRuntimeServices, RawSystemTable, Revision},
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
            ) -> Status {
                Status::SUCCESS
            }

            unsafe extern "efiapi" fn output_string(
                this: *mut RawSimpleTextOutput,
                string: *const Char16,
            ) -> Status {
                Status::SUCCESS
            }

            unsafe extern "efiapi" fn clear_screen(this: *mut RawSimpleTextOutput) -> Status {
                Status::SUCCESS
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
            unsafe extern "efiapi" fn set_mode(this: *mut RawGraphicsOutput, mode: u32) -> Status {
                Status::DEVICE_ERROR
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
                console_in_handle: EfiHandle::null(),
                con_in: null_mut(),
                console_out_handle: EfiHandle::null(),
                con_out: null_mut(),
                console_err_handle: EfiHandle::null(),
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
            let mut vendor = MOCK_VENDOR.encode_utf16().chain([0]).collect::<Vec<u16>>();
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

            system.boot_services = addr_of_mut!(*boot).cast();
            system.runtime_services = addr_of_mut!(*run).cast();
            system.con_out = addr_of_mut!(*out).cast();
            // system.firmware_vendor = addr_of!(vendor[0]);
            system.firmware_vendor = vendor.as_ptr().cast_mut();

            system.header.crc32 = {
                let mut digest = CRC.digest();
                // Safety: We ensure in the definition that there is no uninit padding.
                unsafe { digest.update(to_bytes(&*system)) };
                digest.finalize()
            };

            (
                system,
                vec![
                    //
                    boot,
                    out,
                    run,
                    Box::new(vendor),
                ],
            )
        }

        use imps::*;
        mod imps {
            use core::ffi::c_void;

            use super::*;

            pub static mut MOCK_GOP: RawGraphicsOutput = mock_gop();

            pub unsafe extern "efiapi" fn locate_protocol(
                guid: *mut proto::Guid,
                key: *mut c_void,
                out: *mut *mut c_void,
            ) -> Status {
                let guid = *guid;
                if guid == GraphicsOutput::GUID {
                    out.write(addr_of_mut!(MOCK_GOP) as *mut _);
                    Status::SUCCESS
                } else {
                    out.write(null_mut());
                    Status::NOT_FOUND
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

        // let gop = boot.handle_for::<GraphicsOutput>()?;
        // let gop = boot
        //     .open_protocol::<GraphicsOutput>(gop)?
        //     .ok_or(Status::UNSUPPORTED)?;
        // let _ = gop.set_mode(69);
        // panic!("{gop:?}");

        #[cfg(no)]
        {
            let img = boot.handle_protocol::<LoadedImage>(handle)?;
            let dev = img
                .map(|img| {
                    info!("img: path = {}", img.file_path().unwrap());
                    img
                })
                .and_then(|f| f.device())
                .ok_or(Status::INVALID_PARAMETER)?;
        }
        Ok(())
    }

    const IMAGE: EfiHandle = unsafe { EfiHandle::new(69420 as *mut _) };

    /// This test sets up a mock UEFI environment for the purposes of running
    /// our wrappers and unsafe code through MIRI where possible, in as
    /// close an environment to reality as possible.
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

            let mut evil = Header {
                signature: RawSystemTable::SIGNATURE,
                revision: RawSystemTable::REVISION,
                size: 24,
                crc32: 0,
                reserved: 0,
            };

            let mut digest = CRC.digest();

            digest.update(&evil.signature.to_ne_bytes());
            digest.update(&evil.revision.0.to_ne_bytes());
            digest.update(&evil.size.to_ne_bytes());
            digest.update(&0u32.to_ne_bytes());
            digest.update(&evil.reserved.to_ne_bytes());
            evil.crc32 = digest.finalize();

            let st = (&mut evil) as *mut _ as *mut _;

            let ret = efi_main(IMAGE, st);

            if !ret.is_error() {
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
            // Re-boxing them causes the error but not directly??
            // See: The commit that added this comment for details
            // forget(_box);
        }
        Ok(())
    }
}
