#![allow(unused_imports, unused_variables, dead_code)]
use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    AttributeArgs,
    Error,
    Ident,
    ItemFn,
    Lit,
    Meta,
    Pat,
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

    let mut krate = format_ident!("uefi");
    for arg in args {
        match arg {
            syn::NestedMeta::Meta(Meta::NameValue(m)) => {
                if let Some(i) = m.path.get_ident() {
                    if i == "crate" {
                        if let Lit::Str(s) = m.lit {
                            krate = format_ident!("{}", s.value());
                        } else {
                            errors.push(Error::new(m.lit.span(), "Expected string literal"));
                        }
                    } else {
                        errors.push(Error::new(m.span(), format!("Unexpected argument `{}`", i)));
                    }
                }
            }
            syn::NestedMeta::Meta(m) => {
                let name = m.path().get_ident();
                let span = m.span();
                if let Some(name) = name {
                    errors.push(Error::new(
                        span,
                        format!("Unexpected argument `{:?}`", name),
                    ));
                } else {
                    errors.push(Error::new(span, format!("Unexpected argument `{:?}`", m)));
                }
            }
            syn::NestedMeta::Lit(_) => todo!(),
        }
    }

    let sig = &input.sig;
    let ident = &sig.ident;
    let attrs = &input.attrs;
    let body = &input.block;
    if !attrs.is_empty() {
        // panic!("Had {} attributes and expected zero", input.attrs.len());
    }
    let params = &sig.inputs;
    if params.is_empty() {
        errors.push(Error::new(
            sig.span(),
            format!("Incorrect function signature, expected `fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>`\
\n\
Try `fn {}(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>`
",ident),
        ));
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
            match unexpected {
                syn::FnArg::Receiver(_) => errors.push(Error::new(span, "Unexpected argument")),
                syn::FnArg::Typed(n) => {
                    if let Pat::Ident(i) = &*n.pat {
                        errors.push(Error::new(
                            span,
                            format!("Unexpected argument: `{}`", i.ident),
                        ));
                    } else {
                        errors.push(Error::new(span, "Unexpected argument"))
                    }
                }
            }
        }
    }

    for a in params.iter().take(2) {
        match a {
            syn::FnArg::Receiver(a) => {
                errors.push(Error::new(a.span(), "Cannot be a method"));
            }
            syn::FnArg::Typed(a) => {}
        };
    }

    // NOTE: Keep `MainCheck` up with actual definition.
    // This is breaking to change.
    let span = sig.span();
    let chk = quote! {
        type MainCheck = fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;

        const _chk: MainCheck = #ident;
    };

    let expanded = quote! {
        const _: () = {
            use #krate::{
                EfiHandle,
                SystemTable,
                table::Boot,
                error,
                error::EfiStatus,
            };

            #chk

            #[no_mangle]
            pub static __INTERNAL_PRIVATE_NUEFI_MACRO_SIG_VERIFIED: Option<bool> = Some(false);

            #[no_mangle]
            fn __internal__nuefi__main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
                #ident(handle, table)
            }
        };

        #input
    };

    if let Some(e) = errors.into_iter().reduce(|mut acc, e| {
        acc.combine(e);
        acc
    }) {
        let e = e.into_compile_error();
        TokenStream::from(quote! {
            #e
            #expanded
        })
    } else {
        TokenStream::from(expanded)
    }
}

fn entry_impl() {
    //
}
