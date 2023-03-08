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
