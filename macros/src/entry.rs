use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input,
    spanned::Spanned,
    AttributeArgs,
    Ident,
    ItemFn,
    Lit,
    Meta,
    MetaList,
    NestedMeta,
    Pat,
    Path,
};

use crate::imp::{krate, CommonOpts, Errors};

/// Options our macro accepts
struct Config {
    /// Common macro arguments
    common: CommonOpts,

    /// Register global alloc
    ///
    /// `entry(alloc)`
    alloc: bool,

    /// Default panic handler
    ///
    /// `entry(panic)`
    panic: bool,

    /// Default alloc error handler
    ///
    /// `entry(alloc_error)`
    alloc_error: bool,

    /// Whether to generate and register a default `UefiLogger`
    ///
    /// - `entry(log)`
    /// - `entry(log(..))`
    log: Option<Log>,
}

impl Config {
    fn new() -> Self {
        Self {
            common: CommonOpts::new(),
            alloc: false,
            panic: false,
            alloc_error: false,
            log: None,
        }
    }
}

/// `entry(log(..))` options
struct Log {
    /// Whether logging is colorful or not
    color: bool,

    /// Whether all targets are enabled
    ///
    /// Mutually exclusive with `targets`
    all: bool,

    /// Enable just these targets
    ///
    /// Mutually exclusive with `all`
    targets: Option<Vec<String>>,

    /// Exclude these targets
    exclude: Option<Vec<String>>,
}

impl Log {
    fn new() -> Self {
        Self {
            //
            color: false,
            all: false,
            targets: None,
            exclude: None,
        }
    }
}

fn log(i: &Ident, list: &MetaList, errors: &mut Errors, opts: &mut Config) -> bool {
    if i == "log" {
        let mut log = Log::new();
        let mut exclude: Vec<String> = Vec::new();
        let mut targets: Vec<String> = Vec::new();

        for a in &list.nested {
            match a {
                NestedMeta::Meta(Meta::Path(p)) => {
                    if let Some(i) = p.get_ident() {
                        if i == "color" {
                            if log.color {
                                errors.push(p.span(), "Duplicate attribute `color`");
                            }
                            log.color = true;
                        } else if i == "all" {
                            if log.targets.is_some() {
                                errors.push(p.span(), "Cannot use `all` and `targets` together");
                            }
                            if log.all {
                                errors.push(p.span(), "Duplicate attribute `all`");
                            }
                            log.all = true;
                        } else {
                            errors.push(i.span(), format!("Unexpected argument `{}`", i));
                        }
                    }
                }
                NestedMeta::Meta(Meta::List(li)) => {
                    if let Some(i) = li.path.get_ident() {
                        if i == "exclude" {
                            if log.exclude.is_some() {
                                errors.push(li.path.span(), "Duplicate attribute `exclude`");
                            } else {
                                for f in &li.nested {
                                    match f {
                                        NestedMeta::Meta(_m) => {
                                            errors.push(
                                                li.span(),
                                                format!("Expected value: {:?}", li.nested),
                                            );
                                        }
                                        NestedMeta::Lit(lit) => match lit {
                                            Lit::Str(lit) => {
                                                exclude.push(lit.value());
                                            }
                                            _v => {
                                                errors.push(
                                                    lit.span(),
                                                    format!("Expected string, got: {:?}", f),
                                                );
                                            }
                                        },
                                    }
                                }
                                log.exclude = Some(exclude.clone());
                            }
                        } else if i == "targets" {
                            if log.all {
                                errors.push(
                                    li.path.span(),
                                    "Cannot use `targets` and `all` together",
                                );
                            }
                            if log.targets.is_some() {
                                errors.push(li.path.span(), "Duplicate attribute `targets`");
                            } else {
                                for f in &li.nested {
                                    match f {
                                        NestedMeta::Meta(_) => {
                                            errors.push(
                                                li.span(),
                                                format!("Expected value: {:?}", li.nested),
                                            );
                                        }
                                        NestedMeta::Lit(lit) => match lit {
                                            Lit::Str(lit) => {
                                                targets.push(lit.value());
                                            }
                                            _ => {
                                                errors.push(
                                                    lit.span(),
                                                    format!("Expected string, got: {:?}", f),
                                                );
                                            }
                                        },
                                    }
                                }
                                log.targets = Some(targets.clone());
                            }
                        } else {
                            errors.push(i.span(), format!("Unexpected argument `{}`", i));
                        }
                    }
                }
                // NestedMeta::Lit(_) => {}
                NestedMeta::Meta(m) => {
                    let path = m.path();
                    let span = m.span();
                    if let Some(i) = path.get_ident() {
                        errors.push(span, format!("Unexpected argument `{}`", i));
                    } else {
                        errors.push(span, format!("Unexpected argument `{:?}`", path));
                    }
                }
                e => {
                    errors.push(e.span(), format!("Unexpected argument `{:?}`", e));
                }
            }
        }

        if opts.log.replace(log).is_some() {
            errors.push(i.span(), "Duplicate attribute `log`");
        }

        true
    } else {
        false
    }
}

fn simple_opts(i: &Ident, path: &Path, errors: &mut Errors, opts: &mut Config) -> bool {
    if i == "log" {
        let log = Log::new();
        if opts.log.replace(log).is_some() {
            errors.push(path.span(), "Duplicate attribute `log`");
        }
        true
    } else if i == "alloc" {
        if opts.alloc {
            errors.push(path.span(), "Duplicate attribute `alloc`");
        }
        opts.alloc = true;
        true
    } else if i == "alloc_error" {
        if opts.alloc_error {
            errors.push(path.span(), "Duplicate attribute `alloc_error`");
        }
        opts.alloc_error = true;
        true
    } else if i == "panic" {
        if opts.panic {
            errors.push(path.span(), "Duplicate attribute `panic`");
        }
        opts.panic = true;
        true
    } else {
        false
    }
}

#[allow(clippy::if_same_then_else)]
fn parse_args(args: &[NestedMeta], errors: &mut Errors, opts: &mut Config) {
    for arg in args {
        match &arg {
            NestedMeta::Meta(Meta::NameValue(m)) => {
                if let Some(i) = m.path.get_ident() {
                    if i == "crate" {
                        errors.push(
                            m.span(),
                            r#"Attribute `crate` expected value. Try `crate("VALUE")`"#,
                        );
                    } else {
                        errors.push(m.span(), format!("Unexpected argument `{}`", i));
                    }
                } else {
                    errors.push(m.span(), format!("Unexpected argument `{:?}`", m.path));
                }
            }
            NestedMeta::Meta(Meta::List(l)) => {
                if let Some(i) = l.path.get_ident() {
                    if log(i, l, errors, opts) {
                    } else if krate(i, l, errors, &mut opts.common) {
                    } else {
                        errors.push(l.span(), format!("Unexpected argument `{}`", i));
                    }
                } else {
                    errors.push(l.span(), format!("Unexpected argument `{:?}`", l.path));
                }
            }
            NestedMeta::Meta(Meta::Path(p)) => {
                if let Some(i) = p.get_ident() {
                    if simple_opts(i, p, errors, opts) {
                    } else {
                        errors.push(p.span(), format!("Unexpected argument `{}`", i));
                    }
                } else {
                    errors.push(p.span(), format!("Unexpected argument `{:?}`", p));
                }
            }
            NestedMeta::Lit(l) => {
                errors.push(l.span(), format!("Unknown literal: `{:?}`", l));
            }
        }
    }
}

pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemFn);
    let mut errors = Errors::new();

    let mut opts = Config::new();

    parse_args(&args, &mut errors, &mut opts);

    let sig = &input.sig;
    let ident = &sig.ident;
    let _attrs = &input.attrs;
    let params = &sig.inputs;
    // TODO: sig.output
    if params.is_empty() {
        errors.push(
            sig.span(),
            // TODO: Only include return if its actually incorrect?
            format!(
                "Incorrect function signature, \
            expected two arguments of types `EfiHandle` and `SystemTable<Boot>`\
\n\
Try `fn {}(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()>`
",
                ident
            ),
        );
    }
    if params.len() == 1 {
        let mut p = params.iter();
        let unexpected = p.next().unwrap();
        let span = unexpected.span();
        errors.push(span, "Missing `table` argument");
    }
    if params.len() > 2 {
        let p = params.iter().skip(2);
        for unexpected in p {
            let span = unexpected.span();
            match unexpected {
                syn::FnArg::Receiver(_) => errors.push(span, "Unexpected argument"),
                syn::FnArg::Typed(n) => {
                    if let Pat::Ident(i) = &*n.pat {
                        errors.push(span, format!("Unexpected argument: `{}`", i.ident));
                    } else {
                        errors.push(span, "Unexpected argument");
                    }
                }
            }
        }
    }

    for a in params.iter().take(2) {
        match a {
            syn::FnArg::Receiver(a) => {
                errors.push(a.span(), "Cannot be a method");
            }
            syn::FnArg::Typed(_) => {
                // NOTE: Apparently not possible to verify types in proc macro?
            }
        };
    }

    let krate = opts.common.krate();

    let panic = if opts.panic {
        quote! {
            const _: () = {
                use #krate::handlers::panic;
                use core::panic::PanicInfo;

                // Helps with faulty rust-analyzer/linking errors
                #[cfg_attr(not(any(test, doctest)), panic_handler)]
                fn handle_panic(info: &PanicInfo) -> ! {
                    panic(info);
                }
            };
        }
    } else {
        quote! {}
    };

    let alloc_error = if opts.alloc_error {
        quote! {
            const _: () = {
                use #krate::handlers::alloc_error;
                use core::alloc::Layout;

                // Helps with faulty rust-analyzer/linking errors
                #[cfg_attr(not(any(test, doctest)), alloc_error_handler)]
                fn handle_alloc(layout: core::alloc::Layout) -> ! {
                    alloc_error(layout);
                }
            };
        }
    } else {
        quote! {}
    };

    let alloc = if opts.alloc {
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

    let log = if let Some(log) = opts.log {
        let exclude = log.exclude.unwrap_or_default();
        let targets = log.targets.unwrap_or_default();
        let color = if log.color {
            quote! {.color();}
        } else {
            quote! {;}
        };
        let color_ty = if log.color {
            quote! {UefiColorLogger}
        } else {
            quote! {UefiLogger}
        };
        let all = if log.all {
            quote! { all() }
        } else {
            quote! { new(&[module_path!(), #(#targets),*]) }
        };
        quote! {{
            #[allow(unused_imports)]
            use #krate::logger::{UefiColorLogger, UefiLogger};
            use ::core::module_path;

            static NUEFI_LOGGER: #color_ty = UefiLogger::#all
                .exclude(&[#(#exclude),*])
                #color

            UefiLogger::init(&NUEFI_LOGGER);
        }}
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

            #[no_mangle]
            pub static __INTERNAL_NUEFI_YOU_MUST_USE_MACRO: Option<bool> = Some(false);

            #[no_mangle]
            pub fn __internal__nuefi__main(handle: EfiHandle, table: SystemTable<Boot>) -> error::Result<()> {
                #log
                #ident(handle, table)
            }
        };

        #input

        #panic

        #alloc

        #alloc_error
    };

    let e = if let Some(e) = errors.combine() {
        e.into_compile_error()
    } else {
        quote! {}
    };

    TokenStream::from(quote! {
        #e
        #expanded
    })
}
