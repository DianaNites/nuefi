use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ext::IdentExt,
    parse_macro_input,
    spanned::Spanned,
    ItemStruct,
    Type,
    TypeGroup,
    TypePath,
};

use crate::{
    compat::AttributeArgs,
    guid::{parse_args, GuidOpts},
    imp::Errors,
};

pub fn proto(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);
    let mut errors: Errors = Errors::new();
    let mut opts = GuidOpts::new();

    parse_args(&args, &mut errors, &mut opts);

    let imp_struct = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // This makes errors really nice
    let error_def = quote! {
        ()
    };

    // Makes nice errors
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

    let krate = opts.common.krate();

    let guid = crate::guid::guid_tokens(&opts.guid, &krate);

    let name = imp_struct.unraw().to_string();

    let expanded = quote! {
        #input

        // #[cfg(no)]
        unsafe impl #impl_generics #krate::nuefi_core::extra::Protocol<'table> for #imp_struct #ty_generics #where_clause {
            #guid

            const NAME: &'static str = #name;

            type Raw = #imp_raw_ty_ident;

            #[inline]
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
