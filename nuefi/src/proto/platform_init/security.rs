//! UEFI PI Security Protocols
//!
//! These are described in the UEFI Platform Initialization Specification
//! Version 1.7, Volume 2, Section 12.9 Security Architectural Protocols

use raw::{RawSecurityArch, RawSecurityArch2};

use crate::{
    nuefi_core::interface,
    proto::{Guid, Protocol},
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("A46423E3-4617-49F1-B9FF-D1BFA9115839")]
    SecurityArch(RawSecurityArch)
);

impl<'table> SecurityArch<'table> {
    //
}

interface!(
    #[Protocol("94AB2F58-1438-4EF1-9152-18941A3A0E68")]
    SecurityArch2(RawSecurityArch2)
);

impl<'table> SecurityArch2<'table> {
    //
}
