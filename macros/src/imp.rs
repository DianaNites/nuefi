use std::fmt::Display;

use quote::{__private::Span, format_ident};
use syn::{spanned::Spanned, Error, Ident, Lit, Meta, MetaList, NestedMeta};

/// Options common to our macro, such as `crate`
#[derive(Debug)]
pub struct CommonOpts {
    /// `nuefi` crate name
    ///
    /// `entry(crate("name"))`
    krate: Option<Ident>,
}

impl CommonOpts {
    pub const fn new() -> Self {
        Self {
            //
            krate: None,
        }
    }

    /// Ident for our crate
    pub fn krate(&self) -> Ident {
        self.krate.clone().unwrap_or(format_ident!("nuefi"))
    }
}

/// Error stack during parsing macro input
pub struct Errors {
    data: Vec<Error>,
}

impl Errors {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Push error onto stack with the provided span
    pub fn push<D: Display>(&mut self, span: Span, msg: D) {
        self.data.push(Error::new(span, msg));
    }

    /// Combine all errors into a single one using [`Error::combine`]
    pub fn combine(self) -> Option<Error> {
        self.data.into_iter().reduce(|mut acc, e| {
            acc.combine(e);
            acc
        })
    }
}

/// Attempt to parse the `crate("name")` attribute argument,
/// returning whether we did so.
pub fn krate(i: &Ident, meta: &MetaList, errors: &mut Errors, opts: &mut CommonOpts) -> bool {
    if i == "crate" {
        if let Some(f) = meta.nested.first() {
            match f {
                NestedMeta::Meta(_) => {
                    errors.push(meta.span(), format!("Expected value: {:?}", meta.nested));
                }
                NestedMeta::Lit(li) => match li {
                    Lit::Str(lit) => match opts.krate {
                        Some(_) => {
                            errors.push(meta.span(), "Duplicate attribute `crate`");
                        }
                        None => {
                            opts.krate.replace(format_ident!("{}", lit.value()));
                        }
                    },
                    _ => {
                        errors.push(meta.nested.span(), "Expected string literal");
                        errors.push(li.span(), "Expected string literal");
                    }
                },
            }
        }
        true
    } else {
        false
    }
}

// #[cfg(no)]
#[allow(clippy::if_same_then_else)]
fn _parse_args<F>(args: &[NestedMeta], errors: &mut Errors, opts: &mut CommonOpts, user: F)
where
    F: FnMut(&NestedMeta, &mut Errors) -> bool,
{
    let mut user = user;
    for arg in args {
        match arg {
            // `arg(val)`
            NestedMeta::Meta(Meta::List(l)) => {
                if let Some(i) = l.path.get_ident() {
                    if krate(i, l, errors, opts) {
                    } else if user(arg, errors) {
                    } else {
                        errors.push(l.span(), format!("Unexpected argument `{:?}`", l.path));
                    }
                } else {
                    errors.push(l.span(), format!("Unexpected argument `{:?}`", l.path));
                }
            }

            nested => {
                if let NestedMeta::Meta(Meta::List(l)) = arg {
                    if let Some(i) = l.path.get_ident() {
                        if krate(i, l, errors, opts) {
                        } else if user(arg, errors) {
                        } else {
                            errors.push(l.span(), format!("Unexpected argument `{:?}`", l.path));
                        }
                    } else {
                        errors.push(l.span(), format!("Unexpected argument `{:?}`", l.path));
                    }
                }
                // Steps
                // Run `f`
                // Errors MUST be in proper order
                // need to parse crate but also check `f`
                // need to run f if not crate match
                //
                // User match expected to return bool
                match nested {
                    // `arg(val)`
                    NestedMeta::Meta(Meta::List(_l)) => {}

                    // `val`
                    NestedMeta::Meta(Meta::Path(_p)) => {}

                    // `val`
                    NestedMeta::Lit(l) => {
                        errors.push(l.span(), format!("Unknown literal: `{:?}`", l));
                    }

                    _ => {}
                }
            }
        }
    }
}
