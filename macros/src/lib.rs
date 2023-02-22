#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Error, ItemFn, Lit, Meta, Pat};

mod entry;
mod proto;

/// The UEFI Entry point
///
/// A function with this attribute must appear once in the entire
/// dependency tree or link errors will result.
///
/// # Options
///
/// - `crate = "name"`.
///     - Changes the root crate used to reference types.
///     Here you can see we changed `uefi` to `nuefi`, but the macro
///     would still use `uefi` and fail to compile.
///     This option solves that problem.
/// - `log`
///     - This enables some [`log`][log] statements to be inserted
///     by the library `efi_main` handler.
/// - `panic`
///     - Whether to generate a `panic_impl` or leave it up to you
/// - `alloc`
///     - Whether to generate a `alloc_error_handler` or leave it up to you.
///     This requires [`#![feature(alloc_error_handler)]`][alloc_err].
/// - `delay(N)`
///     - Enables a delay of `N` seconds before returning to firmware.
///     If this is not specified, there is no delay.
///
/// # Example
///
/// Showing how to use the attribute and some basic options
///
/// ```rust
/// # use nuefi::entry;
/// # use nuefi::EfiHandle;
/// # use nuefi::SystemTable;
/// # use nuefi::table::Boot;
/// # use nuefi::error::Result;
/// // Or through the `package` key in `Cargo.toml`!
/// use uefi as nuefi;
///
/// #[entry(crate = "nuefi", delay(69))]
/// fn e_main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// [log]: <https://crates.io/crates/log>
/// [alloc_err]: <https://doc.rust-lang.org/nightly/unstable-book/language-features/alloc-error-handler.html>
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    entry::entry(args, input)
}

/// A UEFI Protocol
///
/// This is the only valid way to implement the
/// [`uefi::proto::Protocol`][Protocol] trait.
///
/// This macro accepts the GUID as a string literal, in mixed-endian hex.
///
/// The struct this is applied to MUST have been created with the
/// [`uefi::interface`][interface] macro.
/// It is designed to work with this macro.
///
/// # Example
///
/// ```rust
/// # use uefi::interface;
/// # use uefi::Protocol;
/// # pub struct RawMyProtocol;
/// #
/// interface!(
///     #[Protocol("A46423E3-4617-49F1-B9FF-D1BFA9115839")]
///     MyProtocol(RawMyProtocol)
/// );
/// ```
///
/// # Safety
///
/// The GUID MUST be valid for the type signature you provide,
/// otherwise unsafe/undefined type mismatch and confusion will result.
///
/// This macro is only intended to be used by internal developers.
///
/// [Protocol]: ./proto/trait.Protocol.html
/// [interface]: ./macro.interface.html
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    proto::proto(args, input)
}
