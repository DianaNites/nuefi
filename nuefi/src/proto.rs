//! Supported/known UEFI Protocols

use core::{marker::PhantomData, ops::Deref};

use log::error;
use nuuid::Uuid;

use crate::{get_boot_table, EfiHandle};

pub mod console;
pub mod device_path;
pub mod graphics;
pub mod loaded_image;
pub mod media;
pub mod platform_init;
pub mod vendor;

#[allow(dead_code)]
type Void = *mut [u8; 0];

pub type Str16 = *const u16;

/// Defines a UEFI Protocol
///
/// See [`crate::Protocol`] for how to implement this.
/// This is the only safe way to implement this trait.
///
/// # Safety
///
/// This trait is unsafe because an incorrect GUID will
/// lead to type confusion and unsafety for both Rust and UEFI.
pub unsafe trait Protocol<'table> {
    /// Protocol GUID
    const GUID: Guid;

    /// Protocol Name
    const NAME: &'static str;

    /// Raw type of this Protocol
    type Raw;

    /// # Safety
    ///
    /// - Must be a valid, non-null, pointer to an instance of Self::Raw
    #[doc(hidden)]
    unsafe fn from_raw(this: *mut Self::Raw) -> Self;

    fn guid(&self) -> Guid {
        Self::GUID
    }
}

/// UEFI GUID
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(64))]
// #[repr(transparent)]
// FIXME: should be 64-bit aligned?
// This should never be passed by value to UEFI, which means transparent does
// nothing?
#[allow(clippy::undocumented_unsafe_blocks)]
pub struct Guid([u8; 16]);

impl core::fmt::Debug for Guid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let uuid = Uuid::from_bytes_me(self.0);
        f.debug_tuple("Guid") //.
            .field(&self.0)
            .field(&format_args!("[Guid] {uuid}"))
            .finish()
    }
}

impl Guid {
    pub(crate) const fn to_uuid(self) -> Uuid {
        Uuid::from_bytes_me(self.0)
    }

    /// # Safety
    ///
    /// - MUST be a valid protocol GUID
    pub const unsafe fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(nuuid::Uuid::from_bytes_me(bytes).to_bytes())
    }

    /// # Safety
    ///
    /// - MUST be a valid protocol GUID
    /// - MUST only be called by the [`crate::Protocol`] macro
    #[doc(hidden)]
    pub const unsafe fn __from_bytes_protocol(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// # Safety
    ///
    /// - MUST only be called by the [`crate::Protocol`] macro
    #[doc(hidden)]
    pub const unsafe fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// A scope around a [Protocol] that will call
/// [`crate::table::BootServices::close_protocol`] on [Drop]
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

    /// Close this protocol
    pub fn close(self) {}

    /// "Leak" this Protocol
    ///
    /// It can be closed by calling
    /// [`crate::table::BootServices::close_protocol`]
    pub fn leak(self) {
        core::mem::forget(self);
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
