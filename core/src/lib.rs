//! Core definitions and code for Nuefi
//!
//! Provides definitions for [UEFI][spec] types and structures
//!
//! # Design
//!
//! These are intended to be mostly simple raw wrappers,
//! but there may be conveniences or extra information provided.
//! These are not intended to be pure raw definitions, adding nothing else.
//!
//! # Organization
//!
//! The modules in this crate are organized roughly following
//! the organization given in the [spec][spec]
//!
//! - [`base`] contains the core UEFI data types
//! - [`error`] is our own addition, and provides a nice [`Result`]
//! and Error type using [`base::Status`].
//! - [`table`] contains the various System Tables
//! - [`extra`] contains various "extra" things, types and trait implementations
//!   that make working with UEFI nice, but are not part of UEFI
//! - [`proto`] contains the various UEFI Protocol, organized roughly
//! following the sidebar for the [HTML Spec][spec]
//! - [`handlers`] contains the implementations for `panic` and `alloc_error`
//!   used by the [`entry`] macro
//!
//! # References
//!
//! - [UEFI Specification 2.10][spec]
//!
//! [spec]: https://uefi.org/specs/UEFI/2.10/index.html
#![no_std]
extern crate alloc;

pub mod error;
// pub mod handlers;

pub mod base;
pub mod extra;
pub mod table;

#[doc(inline)]
pub use nuefi_macros::*;

pub mod proto;

mod nuefi_core {
    //! Hack required for the GUID macro to work in this crate

    pub use super::*;
}
