//! Syn 2.x removed support for Rust attributes, so re-implement the basic
//! syntax support that was for some reason removed.

use proc_macro2::{Punct, Spacing, Span};
use quote::{ToTokens, TokenStreamExt};

#[derive(Debug, Clone)]
pub enum NestedMeta {
    Meta(syn::Meta),
    Lit(syn::Lit),
}

impl ToTokens for NestedMeta {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            NestedMeta::Meta(t) => t.to_tokens(tokens),
            NestedMeta::Lit(t) => t.to_tokens(tokens),
        }
    }
}

impl syn::parse::Parse for NestedMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::ext::IdentExt;
        if input.peek(syn::Lit) && !(input.peek(syn::LitBool) && input.peek2(syn::Token![=])) {
            input.parse().map(NestedMeta::Lit)
        } else if input.peek(syn::Ident::peek_any)
            || input.peek(syn::Token![::]) && input.peek3(syn::Ident::peek_any)
        {
            input.parse().map(NestedMeta::Meta)
        } else {
            Err(input.error("expected identifier or literal"))
        }
    }
}

type AttrArgs = syn::punctuated::Punctuated<NestedMeta, syn::Token![,]>;

#[derive(Debug, Clone)]
pub struct AttributeArgs {
    pub attributes: Vec<NestedMeta>,
    pub span: Span,
}

impl Default for AttributeArgs {
    fn default() -> Self {
        Self {
            attributes: Default::default(),
            span: Span::mixed_site(),
        }
    }
}

impl IntoIterator for AttributeArgs {
    type Item = NestedMeta;

    type IntoIter = std::vec::IntoIter<NestedMeta>;

    fn into_iter(self) -> Self::IntoIter {
        self.attributes.into_iter()
    }
}

impl ToTokens for AttributeArgs {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        for arg in &self.attributes {
            arg.to_tokens(tokens);
            tokens.append(Punct::new(',', Spacing::Alone));
        }
    }
}

impl syn::parse::Parse for AttributeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let args = AttrArgs::parse_terminated(input)?;
        // let args = AttrArgs::parse_separated_nonempty(input)?;
        Ok(Self {
            attributes: args.into_iter().collect(),
            span,
        })
    }
}
