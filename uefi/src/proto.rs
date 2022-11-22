//! Supported/known UEFI Protocols

use core::{
    fmt::{self, Write},
    marker::PhantomData,
    ops::Deref,
};

use log::error;

use crate::{
    error::{EfiStatus, Result},
    get_boot_table,
    util::interface,
    EfiHandle,
};

pub mod console;
pub mod device_path;
pub mod loaded_image;
pub mod vendor;

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

/// A scope around a [Protocol] that will call
/// [`crate::BootServices::close_protocol`] on [Drop]
pub struct Scope<'table, Proto: Protocol<'table>> {
    proto: Proto,
    phantom: PhantomData<&'table mut Proto>,
    handle: EfiHandle,
    agent: EfiHandle,
    controller: Option<EfiHandle>,
}

impl<'table, Proto: Protocol<'table>> Scope<'table, Proto> {
    pub fn new(
        proto: Proto,
        handle: EfiHandle,
        agent: EfiHandle,
        controller: Option<EfiHandle>,
    ) -> Self {
        Self {
            proto,
            phantom: PhantomData,
            handle,
            agent,
            controller,
        }
    }
}

impl<'table, Proto: Protocol<'table>> Deref for Scope<'table, Proto> {
    type Target = Proto;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.proto
    }
}

impl<'table, Proto: Protocol<'table>> Drop for Scope<'table, Proto> {
    fn drop(&mut self) {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();
            if let Err(e) = boot.close_protocol::<Proto>(self.handle, self.agent, self.controller) {
                error!("Error dropping scoped protocol: {e}");
            }
        } else {
            error!("Tried dropping scoped protocol after boot services");
        }
    }
}
