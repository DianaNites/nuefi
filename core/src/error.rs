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

pub type Result<T> = core::result::Result<T, UefiError>;
// FIXME: Remove this
pub use crate::base::Status as EfiStatus;

/// Represents a UEFI [`Status`][st]
///
/// [st]: crate::base::Status
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct UefiError {
    inner: EfiStatus,
}

impl UefiError {
    /// Create a new [`UefiError`]
    ///
    /// # Panics
    ///
    /// - If `inner` is [`EfiStatus::SUCCESS`]
    #[inline]
    pub const fn new(inner: EfiStatus) -> Self {
        assert!(
            !inner.is_success(),
            "Tried to use UefiError with a Success status code"
        );
        Self { inner }
    }

    /// The [`EfiStatus`] for this error
    #[inline]
    pub const fn status(self) -> EfiStatus {
        self.inner
    }
}

impl From<EfiStatus> for Result<()> {
    #[inline]
    fn from(value: EfiStatus) -> Self {
        if value.is_success() {
            Ok(())
        } else {
            Err(UefiError::new(value))
        }
    }
}

/// Convert [`EfiStatus`] to [`UefiError`]
///
/// Panics if [`EfiStatus`] is [`EfiStatus::SUCCESS`]
impl From<EfiStatus> for UefiError {
    #[inline]
    fn from(value: EfiStatus) -> Self {
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
        UefiError::new(EfiStatus::DEVICE_ERROR)
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
