use nuuid::Uuid;
use proc_macro::TokenStream;
use quote::{quote, ToTokens, __private::Span};
use syn::{
    ext::IdentExt,
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

    guid_span: Span,
}

impl Opts {
    fn new() -> Self {
        Self {
            common: CommonOpts::new(),
            guid: None,
            guid_span: Span::call_site(),
        }
    }
}

#[allow(clippy::if_same_then_else)]
fn parse_args(args: &[NestedMeta], errors: &mut Errors, opts: &mut Opts) {
    for arg in args {
        match &arg {
            NestedMeta::Lit(Lit::Str(lit)) => {
                opts.guid_span = lit.span();
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
pub(crate) fn parse_guid(
    opts: &Guid,
    input: impl Spanned,
    krate: &Ident,
    errors: &mut Errors,
) -> impl ToTokens {
    // This makes errors really nice
    let error_def = quote! {const GUID: #krate::nuefi_core::base::Guid = unsafe {
        #krate::nuefi_core::base::Guid::new([
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])
    };};

    if let Some(guid) = &opts {
        match Uuid::parse_me(guid) {
            Ok(guid) => {
                let lol = format!("{:?}", guid.to_bytes());
                if let Ok(lol) = syn::parse_str::<ExprArray>(&lol) {
                    quote! {const GUID: #krate::nuefi_core::base::Guid = unsafe {
                        #krate::nuefi_core::base::Guid::new(#lol)
                    };}
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
                errors.push(input.span(), format!("Invalid GUID: {e}"));
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

    let guid = parse_guid(&opts.guid, opts.guid_span, &krate, &mut errors);

    let imp_struct = &input.ident;
    let imp_generics = &input.generics;

    let name = imp_struct.unraw().to_string();

    let guid_imp = quote! {

        // #[cfg(no)]
        unsafe impl #imp_generics #krate::nuefi_core::extra::Entity for #imp_struct #imp_generics {
            #guid

            const NAME: &'static str = #name;
        }
    };

    let expanded = quote! {
        #input

        #guid_imp
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
