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
//! # References
//!
//! - [UEFI Specification 2.10][spec]
//!
//! [spec]: <https://uefi.org/specs/UEFI/2.10/index.html>
#![no_std]

pub mod error;

pub mod base;
