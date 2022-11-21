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
/// # Safety
///
/// This trait is unsafe because an incorrect GUID will
/// lead to type confusion and unsafety for both Rust and UEFI.
pub unsafe trait Protocol {
    const GUID: Guid;
}

/// UEFI GUID
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Guid([u8; 16]);

impl Guid {
    /// # Safety
    ///
    /// - MUST be a valid protocol GUID
    pub const unsafe fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}
