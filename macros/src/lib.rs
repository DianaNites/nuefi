#![allow(unused_imports, unused_variables)]
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    todo!()
}
