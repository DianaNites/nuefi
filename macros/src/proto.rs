use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    ext::IdentExt,
    parse_macro_input,
    spanned::Spanned,
    AttributeArgs,
    Ident,
    ItemStruct,
    Lit,
    Meta,
    NestedMeta,
    Type,
    TypeGroup,
    TypePath,
};

use crate::imp::Errors;

fn parse_args(
    args: &[NestedMeta],
    errors: &mut Errors,
    krate: &mut Ident,
    guid: &mut Option<String>,
) {
    for arg in args {
        match &arg {
            syn::NestedMeta::Meta(Meta::NameValue(m)) => {
                if let Some(i) = m.path.get_ident() {
                    if i == "crate" {
                        if let Lit::Str(s) = &m.lit {
                            *krate = format_ident!("{}", s.value());
                        } else {
                            errors.push(m.lit.span(), "Expected string literal");
                        }
                    } else {
                        errors.push(m.span(), format!("Unexpected argument `{}`", i));
                    }
                } else {
                    errors.push(m.span(), format!("Unexpected argument `{:?}`", m.path));
                }
            }
            // #[cfg(no)]
            syn::NestedMeta::Meta(m) => {
                let name = m.path().get_ident();
                let span = m.span();
                if let Some(name) = name {
                    errors.push(span, format!("Unexpected argument `{}`", name));
                } else {
                    errors.push(span, format!("Unexpected argument `{:?}`", m));
                }
            }
            syn::NestedMeta::Lit(Lit::Str(lit)) => {
                let s = lit.value();
                // Don't check for UUID validity here, its checked later.
                if guid.replace(s).is_some() {
                    errors.push(lit.span(), "Duplicate GUID attribute");
                }
            }
            syn::NestedMeta::Lit(l) => {
                errors.push(l.span(), format!("Unknown literal: `{:?}`", l));
            }
        }
    }
}

pub fn proto(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);
    let mut errors: Errors = Errors::new();

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

    let match_path = |path: &syn::Path, span, errors: &mut Errors| {
        if let Some(path) = path.get_ident() {
            quote! { #path }
        } else {
            errors.push(
                span,
                "Invalid type (1). This macro MUST only be used with `interface` types",
            );
            error_def.clone()
        }
    };

    let match_group = |elem: &syn::Type, span, errors: &mut Errors| match elem {
        syn::Type::Path(TypePath { path, .. }) => match_path(path, span, errors),
        _ => {
            errors.push(
                span,
                "Invalid type (4). This macro MUST only be used with `interface` types",
            );
            error_def.clone()
        }
    };

    let mut match_ty = |ty: &Type, span| match ty {
        syn::Type::Path(TypePath { path, .. }) => match_path(path, span, &mut errors),

        syn::Type::Ptr(ptr) => match &*ptr.elem {
            syn::Type::Path(TypePath { path, .. }) => match_path(path, span, &mut errors),

            syn::Type::Group(TypeGroup { elem, .. }) => match_group(elem, span, &mut errors),

            _ => {
                errors.push(
                    span,
                    "Invalid type (2). This macro MUST only be used with `interface` types",
                );
                error_def.clone()
            }
        },
        _ => {
            errors.push(
                span,
                "Invalid type (3). This macro MUST only be used with `interface` types",
            );
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
                errors.push(fields.named.span(), "Missing Protocol GUID");
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
                errors.push(fields.unnamed.span(), "Missing Protocol GUID");
                error_def
            }
        }
        syn::Fields::Unit => {
            errors.push(input.fields.span(), "Missing Protocol GUID");
            error_def
        }
    };

    let guid = crate::guid::parse_guid(&guid, &input, &krate, &mut errors);

    let name = imp_struct.unraw().to_string();

    let expanded = quote! {
        #input

        // #[cfg(no)]
        unsafe impl<'table> #krate::proto::Protocol<'table> for #imp_struct #imp_generics {
            #guid

            const NAME: &'static str = #name;

            type Raw = #imp_raw_ty_ident;

            unsafe fn from_raw(this: #imp_first_field) -> Self {
                <#imp_struct>::new(this)
            }
        }
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
