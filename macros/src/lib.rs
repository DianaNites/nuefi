#![allow(unused_imports, unused_variables, dead_code)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// The UEFI Entry point
///
/// A function with this attribute must appear ONLY ONCE in the entire
/// dependency tree.
#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // https://github.com/dtolnay/proc-macro-workshop
    let input = parse_macro_input!(input as ItemFn);
    todo!()
}

fn entry_impl() {
    //
}
