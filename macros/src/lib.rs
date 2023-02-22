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
/// This implements the [`uefi::proto::Protocol`] trait,
/// and is the only valid way to do so.
///
/// The struct this is applied to MUST have been created with the
/// [`uefi::util::interface`] macro.
/// It is designed to work with this macro.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    proto::proto(args, input)
}
