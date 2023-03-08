//! Rust friendly [`Result`][`core::result::Result`] for UEFI errors
//! that is conveniently convertible from [`Status`][st].
//!
//! [`Status`][st] warnings and errors are mapped to [`Err`],
//! and success mapped to [`Ok`]
//!
//! This works nicely with the `?` operator
//!
//! See [`Status`][st] for details.
//!
//! [st]: crate::base::Status

/// UEFI Friendly Rust Result
pub type Result<T> = core::result::Result<T, UefiError>;

/// UEFI Status code re-export for convenience
pub use crate::base::Status;

/// Represents a UEFI [`Status`][st]
///
/// [st]: crate::base::Status
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct UefiError {
    inner: Status,
}

impl UefiError {
    /// Create a new [`UefiError`]
    ///
    /// # Panics
    ///
    /// - If `inner` is [`Status::SUCCESS`]
    #[inline]
    pub const fn new(inner: Status) -> Self {
        assert!(
            !inner.is_success(),
            "Tried to use UefiError with a Success status code"
        );
        Self { inner }
    }

    /// The [`Status`] for this error
    #[inline]
    pub const fn status(self) -> Status {
        self.inner
    }
}

impl From<Status> for Result<()> {
    #[inline]
    fn from(value: Status) -> Self {
        if value.is_success() {
            Ok(())
        } else {
            Err(UefiError::new(value))
        }
    }
}

/// Convert [`Status`] to [`UefiError`]
///
/// Panics if [`Status`] is [`Status::SUCCESS`]
impl From<Status> for UefiError {
    #[inline]
    fn from(value: Status) -> Self {
        assert!(
            !value.is_success(),
            "Tried to construct a successful UefiError"
        );
        UefiError::new(value)
    }
}

/// All [`core::fmt::Write`] failures are treated as
/// [`Status::DEVICE_ERROR`][sdr]
///
/// [sdr]: crate::base::Status::DEVICE_ERROR
impl From<core::fmt::Error> for UefiError {
    #[inline]
    fn from(_: core::fmt::Error) -> Self {
        UefiError::new(Status::DEVICE_ERROR)
    }
}

impl core::fmt::Display for UefiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.status())
    }
}

impl core::fmt::Debug for UefiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UefiError")
            .field("inner", &self.inner)
            .field("[Display]", &format_args!("{}", self.inner))
            .finish()
    }
}

mod imp {
    use super::UefiError;
    pub trait Sealed
    where
        Self: Sized,
    {
    }

    impl<T> Sealed for core::result::Result<Option<T>, UefiError> {}
}

/// Helpful trait to work with [`Result<Option<T>>`]
///
/// This trait is sealed
pub trait ResultOptExt<T>: imp::Sealed {
    /// Ensure this [`Result<Option<T>>`] is [`Ok(None)`] when
    /// [`Status`] is `code`
    fn match_self(self, code: Status) -> core::result::Result<Option<T>, UefiError>;

    /// Ensure this [`Result<Option<T>>`] is [`Ok(None)`] when
    /// [`Status`] is [`Status::UNSUPPORTED`]
    #[inline]
    fn unsupported_opt(self) -> core::result::Result<Option<T>, UefiError> {
        self.match_self(Status::UNSUPPORTED)
    }

    /// Ensure this [`Result<Option<T>>`] is [`Ok(None)`] when
    /// [`Status`] is [`Status::NOT_FOUND`]
    #[inline]
    fn not_found_opt(self) -> core::result::Result<Option<T>, UefiError> {
        self.match_self(Status::NOT_FOUND)
    }
}

impl<T> ResultOptExt<T> for core::result::Result<Option<T>, UefiError> {
    #[inline]
    fn match_self(self, code: Status) -> core::result::Result<Option<T>, UefiError> {
        match self {
            Ok(p) => Ok(p),
            Err(e) => {
                if e.status() == code {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }
}
