//! Defines the supported/known UEFI Protocols
//!
//! UEFI Protocols are how you interact with UEFI firmware, and how firmware
//! interacts with you. Protocols are interface pointers identified by a
//! GUID.
//!
//! Currently, only a subset of the UEFI API is implemented.

use crate::base::Guid;

pub mod device_path;

/// Defines a UEFI Protocol
///
/// See [`crate::Protocol`] for how to implement this.
/// This is the only safe way to implement this trait.
///
/// # Safety
///
/// This trait is unsafe because an incorrect GUID or type specification will
/// lead to type confusion and unsafety for both Rust and UEFI.
/// Unsafe code relies heavily on this invariant
pub unsafe trait Protocol<'table> {
    /// Protocol GUID
    ///
    /// # Safety
    ///
    /// - This must be the matching [`Guid`] uniquely identifying the type
    ///   [`Protocol::Raw`]
    const GUID: Guid;

    /// Protocol Name
    const NAME: &'static str;

    /// Raw type of this Protocol
    ///
    /// # Safety
    ///
    /// - This must be the C struct definition from the UEFI Specification
    ///   describing this protocol
    type Raw;

    /// Wrap `Self` around a [`Protocol`] instance `this`
    ///
    /// # Safety
    ///
    /// - `this` must be a valid pointer to a firmware instance of
    ///   [`Protocol::Raw`]
    #[doc(hidden)]
    unsafe fn from_raw(this: *mut Self::Raw) -> Self;

    #[inline]
    fn guid(&self) -> Guid {
        Self::GUID
    }
}
