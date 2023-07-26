//! A safe Rust UEFI library that provides a high level environment to safely
//! write applications and interact with firmware.
//!
//! While this library intends to be flexible and provide as much control as
//! possible when needed, it is primarily designed around a safe high level
//! interface.
//!
//! Documentation is provided on which firmware calls are made when on a best
//! effort basis.
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
//! An application that wants to manage panic and allocation features
//! itself can do so
//!
//! ```rust
//! // We need these imports
//! use nuefi::{entry, EfiHandle, SystemTable, Boot, error};
//! use core::fmt::Write;
//!
//! // Generate the UEFI entry point
//! #[entry]
//! fn e_main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
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
#![allow(clippy::len_without_is_empty)]
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

/// UEFI Core types
pub use nuefi_core;

/// Handle to the SystemTable. Uses Acquire/Release
static TABLE: AtomicPtr<RawSystemTable> = AtomicPtr::new(core::ptr::null_mut());

/// Handle to the images [`EfiHandle`]. Uses Relaxed, sync with [`TABLE`]
static HANDLE: AtomicPtr<c_void> = AtomicPtr::new(core::ptr::null_mut());

pub use nuefi_core::base::Handle as EfiHandle;

/// Run the closure `f` with a reference to the global UEFI system table
///
/// If the SystemTable is not in the [`Boot`] state, an error is returned.
pub fn with_boot_table<E, F>(f: F) -> Result<E, error::UefiError>
where
    F: FnOnce(&SystemTable<Boot>) -> E,
{
    if let Some(table) = get_boot_table() {
        Ok(f(&table))
    } else {
        Err(Status::UNSUPPORTED.into())
    }
}

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
/// Uses a user-provided main function of type [`__internal__nuefi__main`] as
/// the library entry-point
///
/// This will be the users entry point, exported by the [`entry`] macro.
/// This is the only way to specify the uefi entry point for a [`nuefi`]
/// program.
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
        proto::{console::SimpleTextOutput, graphics::GraphicsOutput, loaded_image::LoadedImage},
        string::{UcsString, UefiStr, UefiString},
    };

    mod mock;

    #[entry(crate("self"))]
    pub fn mock_main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
        let stdout = table.stdout();
        stdout.reset()?;

        let s = UcsString::new("Test");
        let ss = s.as_slice_with_nul();
        let p = ss.as_ptr();
        let l = ss.len();

        // Safety: `p` and `l` are valid
        let u = unsafe { UefiStr::from_ptr_len(p.cast_mut(), l) };
        stdout.output_string(&u)?;

        let vendor = table.firmware_vendor();

        let boot = table.boot();

        // Safety: Always valid
        let img = unsafe { boot.locate_protocol::<SimpleTextOutput>()? };

        extern crate std;
        std::dbg!(img);

        panic!();

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
    /// our wrappers and unsafe code through Miri where possible, in as
    /// close an environment to reality as possible.
    #[test]
    fn miri() -> Result<()> {
        // let (mut st, _box) = { mock() };
        let mut sys = mock();
        {
            let st = (&mut sys.sys) as *mut _;
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
        }
        Ok(())
    }
}

// FIXME: It isnt appropriate for anything in nuefi really
// to use the global allocator. UEFI requires things be allocated in certain
// ways, but a library user may well want to use an arena or something.
// This might require the nightly allocator API? Might literally be impossible
// otherwise?
