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

// TODO: The handlers need to not accidentally panic themselves
// Everything they use, recursively, needs to ensure this property.

/// Default panic handler
#[doc(hidden)]
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
                let _ = boot.exit(handle, Status::ABORTED);
            }
            let _ = writeln!(
                stdout,
                "Failed to abort on panic. Call to `BootServices::Exit` failed. Handle was {:p}",
                handle_p
            );
        }
    }
    // Uselessly loop if we cant do anything else.
    // Do nothing, I guess?
    // UEFI watchdog will kill us eventually(~5 minutes from boot)
    // This may not actually be possible.
    loop {
        hlt()
    }
}

/// Default alloc error handler
#[doc(hidden)]
pub fn alloc_error(layout: Layout) -> ! {
    panic!("Couldn't allocate {} bytes", layout.size())
}

#[cfg(target_arch = "x86_64")]
fn hlt() {
    // Safety: Valid x86_64 instruction
    unsafe { core::arch::asm!("hlt") };
}
