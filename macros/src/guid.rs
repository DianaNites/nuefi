use nuuid::Uuid;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    AttributeArgs,
    ExprArray,
    Ident,
    ItemStruct,
    Lit,
    Meta,
    NestedMeta,
};

use crate::imp::{krate, CommonOpts, Errors};

pub type Guid = Option<String>;

/// Options our macro accepts
struct Opts {
    /// Common macro arguments
    common: CommonOpts,

    /// GUID macro argument
    ///
    /// `GUID("A46423E3-4617-49F1-B9FF-D1BFA9115839")`
    guid: Guid,
}

impl Opts {
    fn new() -> Self {
        Self {
            common: CommonOpts::new(),
            guid: None,
        }
    }
}

#[allow(clippy::if_same_then_else)]
fn parse_args(args: &[NestedMeta], errors: &mut Errors, opts: &mut Opts) {
    for arg in args {
        match &arg {
            NestedMeta::Lit(Lit::Str(lit)) => {
                if opts.guid.replace(lit.value()).is_some() {
                    errors.push(lit.span(), "Duplicate GUID attribute");
                }
            }
            NestedMeta::Meta(Meta::List(l)) => {
                if let Some(i) = l.path.get_ident() {
                    if krate(i, l, errors, &mut opts.common) {
                    } else {
                        // TODO: Common Errors
                        errors.push(l.span(), format!("Unknown attribute: `{}`", i));
                    }
                } else {
                    // TODO: Common Errors
                    errors.push(l.span(), format!("Unknown attribute: `{:?}`", l));
                }
            }

            NestedMeta::Meta(meta) => {
                let path = meta.path();
                if let Some(ident) = path.get_ident() {
                    errors.push(ident.span(), format!("Unknown attribute: `{}`", ident));
                } else {
                    errors.push(meta.span(), format!("Unknown attribute: `{:?}`", meta));
                }
            }

            NestedMeta::Lit(l) => {
                errors.push(l.span(), format!("Unknown literal: `{:?}`", l));
            }
        }
    }
}

/// Parse a GUID
pub fn parse_guid(
    opts: &Guid,
    input: impl Spanned,
    krate: &Ident,
    errors: &mut Errors,
) -> impl ToTokens {
    // This makes errors really nice
    let error_def = quote! {unsafe {
        #krate::proto::Guid::from_bytes([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])
    }};

    if let Some(guid) = &opts {
        match Uuid::parse_me(guid) {
            Ok(guid) => {
                let lol = format!("{:?}", guid.to_bytes());
                if let Ok(lol) = syn::parse_str::<ExprArray>(&lol) {
                    quote! {unsafe {
                        #krate::proto::Guid::__from_bytes_protocol(#lol)
                    }}
                } else {
                    quote! {
                        compile_error!(
                            "Uh this shouldn't have happened. Syn failed when it shouldn't have.\n\
                            This breaks the macro.\n\
                            This is message brought to you by the Nuefi `GUID` macro.\n\
                            Please direct your bug report there."
                        )

                        #error_def
                    }
                }
            }
            Err(e) => {
                errors.push(guid.span(), format!("Invalid GUID: {e}"));
                error_def
            }
        }
    } else {
        errors.push(input.span(), "Missing Protocol GUID");
        error_def
    }
}

pub fn guid(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);
    let mut errors = Errors::new();
    let mut opts = Opts::new();

    parse_args(&args, &mut errors, &mut opts);

    let krate = opts.common.krate();

    let _guid = parse_guid(&opts.guid, &input, &krate, &mut errors);

    // TODO: GUID Trait in Nuefi
    let expanded = quote! {
        #input
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

#[cfg(no)]
pub fn proto(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);
    let mut errors: Vec<Error> = Vec::new();

    let mut krate = format_ident!("nuefi");
    let mut guid: Option<String> = None;

    parse_args(&args, &mut errors, &mut krate, &mut guid);

    let imp_struct = &input.ident;
    let imp_generics = &input.generics;

    // This makes errors really nice
    let error_def = quote! {
        ()
    };

    // FIXME: Workaround the interface macro Type being `*mut Ty`
    let mut imp_raw_ty_ident = quote! {()};

    let mut match_path = |path: &syn::Path, span, errors: &mut Vec<Error>| {
        if let Some(path) = path.get_ident() {
            quote! { #path }
        } else {
            errors.push(Error::new(
                span,
                "Invalid type (1). This macro MUST only be used with `interface` types",
            ));
            error_def.clone()
        }
    };

    let mut match_group = |elem: &syn::Type, span, errors: &mut Vec<Error>| match elem {
        syn::Type::Path(TypePath { path, .. }) => match_path(path, span, errors),
        _ => {
            errors.push(Error::new(
                span,
                "Invalid type (4). This macro MUST only be used with `interface` types",
            ));
            error_def.clone()
        }
    };

    let mut match_ty = |ty: &Type, span| match ty {
        syn::Type::Path(TypePath { path, .. }) => match_path(path, span, &mut errors),

        syn::Type::Ptr(ptr) => match &*ptr.elem {
            syn::Type::Path(TypePath { path, .. }) => match_path(path, span, &mut errors),

            syn::Type::Group(TypeGroup { elem, .. }) => match_group(elem, span, &mut errors),

            _ => {
                errors.push(Error::new(
                    span,
                    "Invalid type (2). This macro MUST only be used with `interface` types",
                ));
                error_def.clone()
            }
        },
        _ => {
            errors.push(Error::new(
                span,
                "Invalid type (3). This macro MUST only be used with `interface` types",
            ));
            error_def.clone()
        }
    };

    let imp_first_field = match &input.fields {
        syn::Fields::Named(fields) => {
            if let Some(first) = fields.named.first() {
                let ty = &first.ty;
                let i = match_ty(ty, fields.named.span());
                imp_raw_ty_ident = quote! { #i };
                quote! { #ty }
            } else {
                errors.push(Error::new(fields.named.span(), "Missing Protocol GUID"));
                error_def
            }
        }
        syn::Fields::Unnamed(fields) => {
            if let Some(first) = fields.unnamed.first() {
                let ty = &first.ty;
                let i = match_ty(ty, fields.unnamed.span());
                imp_raw_ty_ident = quote! { #i };
                quote! { #ty }
            } else {
                errors.push(Error::new(fields.unnamed.span(), "Missing Protocol GUID"));
                error_def
            }
        }
        syn::Fields::Unit => {
            errors.push(Error::new(input.fields.span(), "Missing Protocol GUID"));
            error_def
        }
    };

    // This makes errors really nice
    let error_def = quote! {unsafe {
        #krate::proto::Guid::from_bytes([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])
    }};

    let guid_bytes = if let Some(guid) = guid {
        match Uuid::parse_me(&guid) {
            Ok(guid) => {
                let lol = format!("{:?}", guid.to_bytes());
                if let Ok(lol) = syn::parse_str::<ExprArray>(&lol) {
                    quote! {unsafe {
                        #krate::proto::Guid::__from_bytes_protocol(#lol)
                    }}
                } else {
                    quote! {
                        compile_error!(
                            "Uh this shouldn't have happened. Syn failed when it shouldn't have.\n\
                            This breaks the macro.\n\
                            This is message brought to you by the Nuefi `Protocol` macro.\n\
                            Please direct your bug report there."
                        )

                        #error_def
                    }
                }
            }
            Err(e) => {
                // TODO: parse args config struct, store GUID lit span, use here.
                errors.push(Error::new(guid.span(), format!("Invalid GUID: {e}")));
                error_def
            }
        }
    } else {
        errors.push(Error::new(input.span(), "Missing Protocol GUID"));
        error_def
    };

    let name = imp_struct.unraw().to_string();

    let expanded = quote! {
        #input

        // #[cfg(no)]
        unsafe impl<'table> #krate::proto::Protocol<'table> for #imp_struct #imp_generics {
            const GUID: #krate::proto::Guid = #guid_bytes;

            const NAME: &'static str = #name;

            type Raw = #imp_raw_ty_ident;

            unsafe fn from_raw(this: #imp_first_field) -> Self {
                <#imp_struct>::new(this)
            }
        }
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
