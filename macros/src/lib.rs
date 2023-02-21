#![allow(unused_imports, unused_variables, dead_code)]
use proc_macro::TokenStream;
use quote::{__private::Span, quote, ToTokens};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    AttributeArgs,
    Error,
    Ident,
    ItemFn,
    Type,
    TypePath,
};

/// The UEFI Entry point
///
/// A function with this attribute must appear ONLY ONCE in the entire
/// dependency tree.
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    // TODO: Crate name for importing.
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemFn);
    let mut errors = Vec::new();

    let sig = input.sig;
    let attrs = input.attrs;
    let body = input.block;
    if !attrs.is_empty() {
        // panic!("Had {} attributes and expected zero", input.attrs.len());
    }
    let params = &sig.inputs;
    if params.is_empty() {
        let span = sig.span();
        let err = Error::new(span, "Missing `handle` and `table` arguments");
        errors.push(err);
    }
    if params.len() == 1 {
        let mut p = params.iter();
        let unexpected = p.next().unwrap();
        let span = unexpected.span();
        let err = Error::new(span, "Missing `table` argument");
        errors.push(err);
    }
    if params.len() > 2 {
        let p = params.iter().skip(2);
        for unexpected in p {
            let span = unexpected.span();
            let err = Error::new(span, "Unexpected argument");
            errors.push(err);
        }
    }

    for a in params.iter().take(2) {
        match a {
            syn::FnArg::Receiver(a) => {
                let span = a.span();
                let err = Error::new(span, "cannot be a method");
                errors.push(err);
            }
            syn::FnArg::Typed(a) => {}
        };
    }

    if let Some(e) = errors.into_iter().reduce(|mut acc, e| {
        acc.combine(e);
        acc
    }) {
        return TokenStream::from(e.into_compile_error());
    }

    // let mut new_sig = sig.clone();
    // new_sig.ident = Ident::new(string, span);

    // TODO: See `args` above. Crate name.
    let expanded = quote! {
        const _: () = {
            use ::uefi::EfiHandle;
            use ::uefi::SystemTable;
            use ::uefi::table::Boot;
            use ::uefi::table::raw::RawSystemTable;
            use ::uefi::error;
            use ::uefi::error::EfiStatus;

            type MainCheck = fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;

            #[no_mangle]
            extern "efiapi" fn efi_main(image: EfiHandle, system_table: *mut RawSystemTable) -> EfiStatus {
                extern "Rust" {
                    fn main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;
                }
                if image.0.is_null() || system_table.is_null() {
                    return EfiStatus::INVALID_PARAMETER;
                }
                // SAFETY: Pointer is valid from firmware
                let valid = unsafe { RawSystemTable::validate(system_table) };
                if let Err(e) = valid {
                    return e.status();
                }
                todo!();
            }
        };
    };

    TokenStream::from(expanded)
}

fn entry_impl() {
    //
}
