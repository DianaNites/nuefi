#![allow(clippy::redundant_clone)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute,
    AttributeArgs,
    Error,
    Ident,
    ItemFn,
    Lit,
    Meta,
    NestedMeta,
    Pat,
    Token,
};

type Args = Punctuated<NestedMeta, Token![,]>;

pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemFn);
    let mut errors = Vec::new();

    let mut krate: Option<Ident> = None;
    let mut exit_prompt = false;
    let mut handle_log = false;
    let mut delay: Option<u64> = None;
    let mut handle_alloc_error = false;
    let mut handle_alloc = false;
    let mut handle_panic = false;
    let mut handle_color = false;

    for arg in args {
        match &arg {
            syn::NestedMeta::Meta(Meta::NameValue(m)) => {
                if let Some(i) = m.path.get_ident() {
                    if i == "crate" {
                        if let Lit::Str(s) = &m.lit {
                            if krate.replace(format_ident!("{}", s.value())).is_some() {
                                errors.push(Error::new(m.span(), "Duplicate attribute `crate`"));
                            }
                        } else {
                            errors.push(Error::new(m.lit.span(), "Expected string literal"));
                        }
                    } else {
                        errors.push(Error::new(m.span(), format!("Unexpected argument `{}`", i)));
                    }
                } else {
                    errors.push(Error::new(
                        m.span(),
                        format!("Unexpected argument `{:?}`", m.path),
                    ));
                }
            }
            syn::NestedMeta::Meta(Meta::List(l)) => {
                if let Some(i) = l.path.get_ident() {
                    if i == "delay" {
                        if let Some(f) = l.nested.first() {
                            match f {
                                syn::NestedMeta::Meta(m) => {
                                    errors.push(Error::new(
                                        l.span(),
                                        format!("Expected value: {:?}", l.nested),
                                    ));
                                }
                                syn::NestedMeta::Lit(li) => match li {
                                    Lit::Int(lit) => {
                                        if let Ok(lit) = lit.base10_parse::<u64>() {
                                            if delay.replace(lit).is_some() {
                                                errors.push(Error::new(
                                                    l.span(),
                                                    "Duplicate attribute `delay`",
                                                ));
                                            }
                                        }
                                    }
                                    v => {
                                        errors.push(Error::new(
                                            li.span(),
                                            format!("Expected integer, got: {:?}", f),
                                        ));
                                    }
                                },
                            }
                        } else {
                            errors.push(Error::new(
                                l.span(),
                                format!("Expected value: {:?}", l.nested),
                            ));
                        }
                    } else if i == "log" {
                        if handle_log {
                            errors.push(Error::new(i.span(), "Duplicate attribute `log`"));
                            errors.push(Error::new(l.span(), "Mixed `log` and `log(OPTIONS)`"));
                        }
                        for a in &l.nested {
                            match a {
                                NestedMeta::Meta(_) => todo!(),
                                NestedMeta::Lit(_) => todo!(),
                            }
                        }
                        todo!("Log opts");
                        #[cfg(no)]
                        {
                            if i == "color" {
                                if handle_color {
                                    errors
                                        .push(Error::new(p.span(), "Duplicate attribute `color`"));
                                }
                                handle_color = true;
                            }
                        }
                    } else {
                        errors.push(Error::new(
                            l.span(),
                            format!("Unexpected argument `{:?}`", l.path),
                        ));
                    }
                } else if let Some(i) = l.path.get_ident() {
                    errors.push(Error::new(l.span(), format!("Unexpected argument `{}`", i)));
                } else {
                    errors.push(Error::new(
                        l.span(),
                        format!("Unexpected argument `{:?}`", l.path),
                    ));
                }
            }
            syn::NestedMeta::Meta(m @ Meta::Path(p)) =>
            {
                #[allow(clippy::if_same_then_else)]
                if let Some(i) = p.get_ident() {
                    if i == "exit_prompt" {
                        if exit_prompt {
                            errors.push(Error::new(p.span(), "Duplicate attribute `exit_prompt`"));
                        }
                        exit_prompt = true;
                    } else if i == "log" {
                        if handle_log {
                            errors.push(Error::new(p.span(), "Duplicate attribute `log`"));
                        }
                        handle_log = true;
                    } else if i == "alloc" {
                        if handle_alloc {
                            errors.push(Error::new(p.span(), "Duplicate attribute `alloc`"));
                        }
                        handle_alloc = true;
                    } else if i == "alloc_error" {
                        if handle_alloc_error {
                            errors.push(Error::new(p.span(), "Duplicate attribute `alloc_error`"));
                        }
                        handle_alloc_error = true;
                    } else if i == "panic" {
                        if handle_panic {
                            errors.push(Error::new(p.span(), "Duplicate attribute `panic`"));
                        }
                        handle_panic = true;
                    } else if i == "delay" {
                        errors.push(Error::new(
                            p.span(),
                            "Attribute `delay` expected value. Try `delay(VALUE)`",
                        ));
                    } else {
                        errors.push(Error::new(
                            p.span(),
                            format!("Unexpected argument `{:?}`", p),
                        ));
                    }
                } else if let Some(i) = p.get_ident() {
                    errors.push(Error::new(p.span(), format!("Unexpected argument `{}`", i)));
                } else {
                    errors.push(Error::new(
                        p.span(),
                        format!("Unexpected argument `{:?}`", p),
                    ));
                }
            }
            #[cfg(no)]
            syn::NestedMeta::Meta(m) => {
                let name = m.path().get_ident();
                let span = m.span();
                if let Some(name) = name {
                    errors.push(Error::new(span, format!("Unexpected argument `{}`", name)));
                } else {
                    errors.push(Error::new(span, format!("Unexpected argument `{:?}`", m)));
                }
            }
            syn::NestedMeta::Lit(l) => {
                errors.push(Error::new(l.span(), format!("Unknown literal: `{:?}`", l)));
            }
        }
    }

    let sig = &input.sig;
    let ident = &sig.ident;
    let attrs = &input.attrs;
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
            syn::FnArg::Typed(_) => {
                // NOTE: Apparently not possible to verify types in proc macro?
            }
        };
    }

    // NOTE: Keep `MainCheck` up with actual definition.
    // This is breaking to change.
    let chk = quote! {
        type MainCheck = fn(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>;

        const _chk: MainCheck = #ident;
    };

    let exit_dur = if let Some(d) = delay {
        quote! {
            Some(#d)
        }
    } else {
        quote! {
            None
        }
    };

    let krate = krate.unwrap_or(format_ident!("nuefi"));

    let panic = if handle_panic {
        quote! {
            const _: () = {
                use #krate::handlers::panic;
                use core::panic::PanicInfo;

                // Helps with faulty rust-analyzer/linking errors
                #[cfg_attr(not(test), panic_handler)]
                fn handle_panic(info: &PanicInfo) -> ! {
                    panic(info);
                }
            };
        }
    } else {
        quote! {}
    };

    let alloc_error = if handle_alloc_error {
        quote! {
            const _: () = {
                use #krate::handlers::alloc_error;
                use core::alloc::Layout;

                // Helps with faulty rust-analyzer/linking errors
                #[cfg_attr(not(test), alloc_error_handler)]
                fn handle_alloc(layout: core::alloc::Layout) -> ! {
                    alloc_error(layout);
                }
            };
        }
    } else {
        quote! {}
    };

    let alloc = if handle_alloc {
        quote! {
            const _: () = {
                use #krate::mem::UefiAlloc;

                #[global_allocator]
                static NUEFI_ALLOC: UefiAlloc = UefiAlloc::new();
            };
        }
    } else {
        quote! {}
    };

    let log = if handle_log {
        quote! {
            const _: () = {
                use #krate::logger::{UefiColorLogger, UefiLogger};
                // use ::core::module_path;

                static NUEFI_LOGGER: UefiColorLogger = UefiLogger::new(&[module_path!(), "nuefi"])
                    .exclude(&["nuefi::mem"])
                    .color();
            };
        }
    } else {
        quote! {}
    };

    // NOTE: Macro can/should/MUST do linker hacks to
    // ensure persistent runtime panic/alloc_error hooks
    // that way we can allow them to be overridden, and free boot memory.
    // Suspect just need link_section

    let expanded = quote! {
        const _: () = {
            use #krate::{
                EfiHandle,
                SystemTable,
                table::Boot,
                error,
            };

            #chk

            #[no_mangle]
            pub static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool> = Some(false);

            #[no_mangle]
            pub static __INTERNAL_NUEFI_EXIT_DURATION: Option<u64> = #exit_dur;

            #[no_mangle]
            pub fn __internal__nuefi__main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
                #ident(handle, table)
            }
        };

        #input

        #panic

        #alloc

        #alloc_error
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
