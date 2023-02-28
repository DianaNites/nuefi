//! Nuefi internal semver-exempt unstable macro crate
use proc_macro::TokenStream;

mod entry;
mod guid;
mod proto;

mod imp;
use imp::*;

/// The UEFI Entry point
///
/// This attribute marks a function as the UEFI entry point.
/// The function must have two arguments, [`EfiHandle`][EfiHandle] and
/// [`SystemTable<Boot>`][SystemTable], and return [`Result<()>`][Result].
///
/// # Options
///
/// This attribute accepts several options, in the form `entry(option)`,
/// as listed below:
///
/// - `crate("name")`.
///     - Changes the root crate used to reference types.
///     Here you can see we changed `uefi` to `nuefi`, but the macro
///     would still use `uefi` and fail to compile.
///     This option solves that problem.
/// - `log`
///     - Whether to generate and register a default [`log`][log] global logger
///       using [`UefiLogger`][UefiLogger].
///         - By default this will only allow logs from your own crate to be
///           logged.
///     - `all`
///         - Enable all logging without any filtering
///         - This is mutually exclusive with `targets`
///     - `targets("buggy_crate", "buggy::buggy_module", ...)`
///         - Include the logging targets identified by this list, in addition
///           to your own crate.
///         - This is mutually exclusive with `all`
///     - `exclude("overly_verbose_crate", "verbose::module", ...)`
///         - Exclude the logging targets identified by this list.
///     - `color`
///         - Enable colorful logging
/// - `panic`
///     - Whether to generate a `panic_impl` or leave it up to you
/// - `alloc`
///     - Whether to generate a `global_alloc` static or leave it up to you
/// - `alloc_error`
///     - Whether to generate an `alloc_error_handler` or leave it up to you.
///     This requires [`#![feature(alloc_error_handler)]`][alloc_err].
/// - `delay(N)`
///     - Enables a delay of `N` seconds before returning to firmware on errors.
///     If this is not specified, there is no delay.
///
/// # Example
///
/// Showing how to use the attribute and some basic options
///
/// ```rust
/// # use uefi::entry;
/// # use uefi::EfiHandle;
/// # use uefi::SystemTable;
/// # use uefi::table::Boot;
/// # use uefi::error::Result;
/// // Or through the `package` key in `Cargo.toml`!
/// use nuefi as uefi;
///
/// #[entry(crate("uefi"), delay(69))]
/// fn e_main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// # Panic, alloc, and alloc error handlers
///
/// These attributes generate the necessary implementations of these
/// to compile `no_std` code.
///
/// The provided `panic` handler will print to the UEFI stdout,
/// if available and in [`Boot`][Boot] mode, then loops forever.
/// (This will result in the UEFI watchdog, or the user, restarting)
///
/// The provided `alloc_error` handler will panic with information
/// about the failed `Layout`
///
/// The `alloc` attribute will generate the following code
///
/// ```rust,no_run
/// # extern crate alloc;
/// use nuefi::mem::UefiAlloc;
///
/// #[global_allocator]
/// static NUEFI_ALLOC: UefiAlloc = UefiAlloc::new();
/// ```
///
/// # Logger
///
/// The `log` attribute generates code equivalent to the following,
///
/// ```rust
/// # use nuefi::{error::Result, Boot, SystemTable, EfiHandle};
/// use log::info;
///
/// // This
/// use nuefi::entry;
///
/// #[entry(log(
///     targets("targets", "..."),
///     exclude("..."),
///     color
/// ))]
/// fn uefi_main(handle: EfiHandle, table: SystemTable<Boot>) -> Result<()> {
///     info!("Hello, world!");
///     Ok(())
/// }
///
/// // Is the same as this
/// use nuefi::logger::{UefiColorLogger, UefiLogger};
///
/// static NUEFI_LOGGER: UefiColorLogger = UefiLogger::new(
///         &[module_path!(), "targets", "..."],
///     )
///     .exclude(&["..."])
///     .color();
///
/// fn main() {
///     // Called before your code runs
///     UefiLogger::init(&NUEFI_LOGGER);
///     info!("Hello, world!");
/// }
/// ```
///
/// [log]: <https://crates.io/crates/log>
/// [alloc_err]: <https://doc.rust-lang.org/nightly/unstable-book/language-features/alloc-error-handler.html>
/// [UefiLogger]: ./logger/struct.UefiLogger.html
/// [SystemTable]: ./table/struct.SystemTable.html
/// [EfiHandle]: ./struct.EfiHandle.html
/// [Boot]: ./table/struct.Boot.html
/// [Result]: ./error/type.Result.html
// FIXME: Above links for docs.rs? is there a way to portably link?
// ..just make proc macro depend on nuefi?
// cyclic?
// separate types crate?
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
/// # use nuefi::interface;
/// # use nuefi::Protocol;
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

/// Macro to parse a GUID. Accepts a single mixed-endian hex string literal
///
/// This macro accepts the GUID as a string literal, in mixed-endian hex.
///
/// # Example
///
/// ```rust
/// # use nuefi::GUID;
///
/// #[GUID("A46423E3-4617-49F1-B9FF-D1BFA9115839")]
/// struct HasID;
/// ```
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn GUID(args: TokenStream, input: TokenStream) -> TokenStream {
    guid::guid(args, input)
}
