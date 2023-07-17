use nuuid::Uuid;
use proc_macro::TokenStream;
use quote::{quote, ToTokens, __private::Span};
use syn::{ext::IdentExt, parse_macro_input, spanned::Spanned, ExprArray, Ident, ItemStruct, Lit};

use crate::{
    compat::{AttributeArgs, NestedMeta},
    imp::{krate_, CommonOpts, Errors},
};

pub type Guid = Option<String>;

/// Options our macro accepts
#[derive(Debug, Clone)]
pub(crate) struct GuidOpts {
    /// Common macro arguments
    pub common: CommonOpts,

    /// GUID macro argument
    ///
    /// `GUID("A46423E3-4617-49F1-B9FF-D1BFA9115839")`
    pub guid: Guid,

    pub guid_span: Span,
}

impl GuidOpts {
    pub fn new() -> Self {
        Self {
            common: CommonOpts::new(),
            guid: None,
            guid_span: Span::call_site(),
        }
    }
}

/// Parse a GUID
///
/// true if successful, false otherwise.
fn guid_(meta: &NestedMeta, errors: &mut Errors, opts: &mut GuidOpts) -> bool {
    if let NestedMeta::Lit(Lit::Str(lit)) = meta {
        match opts.guid {
            Some(_) => errors.push(lit.span(), "duplicate GUID attribute"),
            None => {
                let v = lit.value();
                let span = lit.span();
                if v.is_empty() {
                    errors.push(span, "GUID cannot be empty");
                } else {
                    match Uuid::parse_le(&v) {
                        Ok(guid) => {
                            opts.guid = Some(format!("{:?}", guid.to_bytes()));
                            opts.guid_span = span;
                        }
                        Err(e) => {
                            errors.push(span, format!("invalid GUID: {e}"));
                        }
                    };
                }
            }
        }
        return true;
    }
    false
}

#[allow(clippy::if_same_then_else)]
pub(crate) fn parse_args(args: &AttributeArgs, errors: &mut Errors, opts: &mut GuidOpts) {
    if args.attributes.is_empty() {
        errors.push(args.span(), "missing GUID");
    }

    for arg in &args.attributes {
        if krate_(arg, errors, &mut opts.common) {
            // continue;
        } else if guid_(arg, errors, opts) {
            // continue;
        } else {
            errors.push(arg.span(), "unknown attribute");
        }
    }
}

/// Parse a GUID
///
/// Returns code like the below,
/// without imports and with the input GUID bytes filled in.
///
/// ```rust,no_run
/// use nuefi_core::base::Guid;
///
/// const GUID: Guid = unsafe {
///       Guid::new([
///           0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///           0x00, 0x00,
///       ])
///   };
/// ```
pub(crate) fn parse_guid(opts: &Guid, krate: &Ident) -> impl ToTokens {
    // This makes errors really nice
    let error_def = quote! {const GUID: #krate::nuefi_core::base::Guid = unsafe {
        #krate::nuefi_core::base::Guid::new([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])
    };};

    if let Some(guid) = &opts {
        let guid = syn::parse_str::<ExprArray>(guid).unwrap();
        quote! {
            const GUID: #krate::nuefi_core::base::Guid = unsafe {
                #krate::nuefi_core::base::Guid::new(#guid)
            };
        }
    } else {
        quote! {#error_def}
    }
}

pub fn guid(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input = parse_macro_input!(input as ItemStruct);
    let mut errors = Errors::new();
    let mut opts = GuidOpts::new();

    parse_args(&args, &mut errors, &mut opts);

    let krate = opts.common.krate();

    let guid = parse_guid(&opts.guid, &krate);

    let imp_struct = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let name = imp_struct.unraw().to_string();

    let guid_imp = quote! {
        impl #impl_generics #imp_struct #ty_generics #where_clause {
            /// GUID of the protocol
            #guid
        }

        unsafe impl #impl_generics #krate::nuefi_core::extra::Entity for #imp_struct #ty_generics #where_clause {
            /// GUID of the protocol
            #guid

            /// Name of the protocol
            const NAME: &'static str = #name;
        }
    };

    let expanded = quote! {
        #input

        #guid_imp
    };

    let e = errors
        .combine()
        .map(|e| e.into_compile_error())
        .unwrap_or(quote! {});

    TokenStream::from(quote! {
        #e
        #expanded
    })
}
