//! Alloc and panic handlers
use core::{
    alloc::Layout,
    fmt::Write,
    panic::PanicInfo,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::get_boot_table;

type AllocFn = fn(Layout) -> !;
type PanicFn = fn(&PanicInfo) -> !;

// Helps with faulty rust-analyzer/linking errors
// #[cfg_attr(not(any(test, special_test)), panic_handler)]
pub fn panic(info: &PanicInfo) -> ! {
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

// Helps with faulty rust-analyzer/linking errors
// #[cfg_attr(not(any(test, special_test)), alloc_error_handler)]
// #[cfg_attr(not(test), alloc_error_handler)]
pub fn alloc_error(layout: Layout) -> ! {
    // Safety: We ensure elsewhere that ALLOC_HANDLER is never set to an improper
    // pointer
    let alloc = ALLOC_HANDLER.load(Ordering::Relaxed);
    let alloc = unsafe { alloc.as_ref() };
    if let Some(Some(alloc)) = alloc {
        // Safety: Above
        let alloc = unsafe { alloc.as_ref() };
        alloc(layout);
    } else if let Some(None) = alloc {
        // Handler overridden, but to do nothing, so do nothing, I guess?
        // UEFI watchdog will kill us eventually(~5 minutes from boot)
        // This may not actually be possible.
        loop {}
    } else {
        panic!("Couldn't allocate {} bytes", layout.size())
    }
}

/// Alloc error handler pointer
///
/// This is public for proc macro reasons and you should not touch this,
/// consider it unsafe.
#[doc(hidden)]
pub static ALLOC_HANDLER: AtomicPtr<Option<NonNull<AllocFn>>> =
    AtomicPtr::new(core::ptr::null_mut());

/// Panic handler pointer
///
/// This is public for proc macro reasons and you should not touch this,
/// consider it unsafe.
#[doc(hidden)]
pub static PANIC_HANDLER: AtomicPtr<Option<NonNull<PanicFn>>> =
    AtomicPtr::new(core::ptr::null_mut());
