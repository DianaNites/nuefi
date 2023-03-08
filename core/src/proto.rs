//! Defines the supported/known UEFI Protocols
//!
//! UEFI Protocols are how you interact with UEFI firmware, and how firmware
//! interacts with you. Protocols are interface pointers identified by a
//! GUID.
//!
//! Currently, only a subset of the UEFI API is implemented.

// FIXME: Ugly hack for GUID macro
pub use crate::{base::Guid, extra::Entity};
