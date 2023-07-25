//! Extra things, types and trait implementations and things
//! that make working with UEFI nice, but are not part of UEFI

use crate::base::Guid;

/// Identifies an entity within UEFI, such as a [`ConfigTable`][ct].
///
/// You shouldn't need to use this, see more
/// specific traits like [`ConfigTable`][ct] instead.
///
/// See the [`GUID`][gm] macro for how to implement this.
///
/// # Safety
///
/// This trait is unsafe because if the GUID you provide is wrong, you can
/// cause UB and confuse what we/UEFI thinks types are.
///
/// You must ensure the GUID is correct for whatever entity you are
/// representing, or else you will cause type confusion.
///
/// [gm]: crate::GUID
/// [ct]: crate::table::config::ConfigTable
pub unsafe trait Entity {
    /// Entity GUID
    const GUID: Guid;

    /// Entity Name
    const NAME: &'static str;

    /// Entity Name
    fn name() -> &'static str {
        Self::NAME
    }

    /// Entity GUID
    fn guid() -> Guid {
        Self::GUID
    }
}

/// Defines a UEFI Protocol
///
/// See [`crate::Protocol`] for how to implement this.
/// This is the only safe way to implement this trait.
///
/// # Safety
///
/// This trait is unsafe because an incorrect GUID or type specification will
/// lead to type confusion and unsafety for both Rust and UEFI.
///
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
    /// # Derive
    ///
    /// The derive macro expects `fn new(*mut Protocol::Raw)` to exist.
    ///
    /// This should come from the `interface` macro
    ///
    /// # Safety
    ///
    /// - `this` must be a valid pointer to an instance of [`Protocol::Raw`]
    ///   that will live for `'table`
    #[doc(hidden)]
    unsafe fn from_raw(this: *mut Self::Raw) -> Self;

    #[inline]
    fn guid(&self) -> Guid {
        Self::GUID
    }

    #[inline]
    fn name(&self) -> &'static str {
        Self::NAME
    }
}
