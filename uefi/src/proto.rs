//! Supported/known UEFI Protocols

use core::fmt::{self, Write};

use crate::{
    error::{EfiStatus, Result},
    util::interface,
};

pub mod console;
pub mod device_path;
pub mod loaded_image;

#[allow(dead_code)]
type Void = *mut [u8; 0];

pub type Str16 = *const u16;

/// Defines a UEFI Protocol
///
/// # Safety
///
/// This trait is unsafe because an incorrect GUID will
/// lead to type confusion and unsafety for both Rust and UEFI.
pub unsafe trait Protocol<'table> {
    /// Protocol GUID
    const GUID: Guid;

    /// # Safety
    ///
    /// - MUST be library author.
    #[doc(hidden)]
    unsafe fn from_raw(this: *mut u8) -> Self;
}

/// UEFI GUID
#[derive(Clone, Copy)]
#[repr(C, align(64))]
// #[repr(transparent)]
// FIXME: should be 64-bit aligned?
// This should never be passed by value to UEFI, which means transparent does
// nothing?
pub struct Guid([u8; 16]);

impl Guid {
    /// # Safety
    ///
    /// - MUST be a valid protocol GUID
    pub const unsafe fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(nuuid::Uuid::from_bytes_me(bytes).to_bytes())
    }
}
