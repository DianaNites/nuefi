extern crate std;
use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    alloc::Layout,
    any::Any,
    fmt::Debug,
    mem::size_of,
    ptr::{addr_of, addr_of_mut, null_mut},
};

use nuefi_core::{
    base::{Char16, Guid},
    table::{Header, CRC},
};

use self::{boot::MockBoot, console::MockConsole, system::System};
use crate::{
    error::Status,
    proto::{
        self,
        console::{raw::RawSimpleTextOutput, SimpleTextOutput},
        graphics::{raw::RawGraphicsOutput, GraphicsOutput},
        Protocol,
    },
    string::UcsString,
    table::{
        raw::{RawBootServices, RawRuntimeServices, RawSystemTable, Revision},
        BootServices,
    },
    EfiHandle,
};

mod boot;
mod console;
mod system;

/// # Safety:
///
/// `T` must not have uninit padding.
const unsafe fn to_bytes<T>(this: &T) -> &[u8] {
    // Safety: `this` is valid by definition
    // Lifetime is bound to `this`
    unsafe { core::slice::from_raw_parts(this as *const T as *const u8, size_of::<T>()) }
}

/// Create mock implementations of a SystemTable and a few protocols
/// to aid testing of the basic interactions
///
/// This especially aids in miri and can help ensure that our wrappers are
/// memory safe, assuming a suitably correct mock and compliant UEFI system.
///
/// Or not so compliant, there are some (debug) checks.
pub fn mock() -> Box<System> {
    let mut sys = System::new();
    let vendor = &mut sys.vendor;
    let system = &mut sys.sys;
    let boot = &mut sys.boot;
    let run = &mut sys.run;

    system.header.crc32 = {
        let mut digest = CRC.digest();
        // Safety: We ensure in the definition that there is no uninit padding.
        unsafe { digest.update(to_bytes(&*system)) };
        digest.finalize()
    };

    sys
}
