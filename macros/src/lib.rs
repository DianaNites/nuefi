#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Error, ItemFn, Lit, Meta, Pat};

mod entry;
mod proto;

/// The UEFI Entry point
///
/// A function with this attribute must appear ONLY ONCE in the entire
/// dependency tree.
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    entry::entry(args, input)
}

/// A UEFI Protocol
///
/// This is the only valid way to implement the [`uefi::proto::Protocol`] trait.
///
/// This macro accepts the GUID as a string literal, in mixed-endian hex.
///
/// The struct this is applied to MUST have been created with the
/// [`uefi::util::interface`] macro.
/// It is designed to work with this macro.
///
/// # Example
///
/// ```rust
/// # use uefi::interface;
/// # use uefi::Protocol;
/// # pub struct RawMyProtocol;
///
/// interface!(
///     #[Protocol("A46423E3-4617-49F1-B9FF-D1BFA9115839")]
///     MyProtocol(RawMyProtocol)
/// );
/// ```
///
/// # Safety
///
/// The GUID MUST be valid for the type signature you provide,
/// otherwise unsafe/undefined type mismatch and confusion will result.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    proto::proto(args, input)
}
