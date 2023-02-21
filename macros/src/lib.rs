#![allow(unused_imports, unused_variables, dead_code)]
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Error, ItemFn};

/// The UEFI Entry point
///
/// A function with this attribute must appear ONLY ONCE in the entire
/// dependency tree.
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemFn);

    let sig = input.sig;
    let attrs = input.attrs;
    if !attrs.is_empty() {
        // panic!("Had {} attributes and expected zero", input.attrs.len());
    }
    let params = &sig.inputs;
    if params.is_empty() {
        let span = sig.span();
        let err = Error::new(span, "Missing `handle` and `table` parameters");
        return TokenStream::from(err.into_compile_error());
    } else if params.len() != 2 {
        // let span = params.span();
        // let err = Error::new(span, "");
        // return TokenStream::from(err.into_compile_error());
        // return TokenStream::from(quote! {
        //     compile_error!("");
        // });
    }

    let expanded = quote! {
        // ...
    };

    TokenStream::from(expanded)
}

fn entry_impl() {
    //
}
