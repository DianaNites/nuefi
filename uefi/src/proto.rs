//! Supported/known UEFI Protocols

use core::fmt::{self, Write};

use crate::{
    error::{EfiStatus, Result},
    util::interface,
};

pub mod console;

#[allow(dead_code)]
type Void = *mut [u8; 0];

pub type Str16 = *const u16;

/// Defines a UEFI Protocol
///
/// This trait is unsafe because an incorrect GUID will
/// lead to type confusion and unsafety for both Rust and UEFI.
pub unsafe trait Protocol {
    const GUID: [u8; 16];
}
