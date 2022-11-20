//! Error type for all UEFI functions
//!
//! This maps EFI_STATUS warnings and errors to [`Err`], and success to [`Ok`]

pub type Result<T> = core::result::Result<T, UefiError>;

/// Bits in EFI_STATUS
// Just to make it less annoying if we end up supporting 128-bit platforms,
// because iirc rust's usize wont be 128-bit there?
const STATUS_BITS: u32 = usize::BITS;

/// High bit indicating error
const ERROR_BIT: usize = 1 << (STATUS_BITS - 1);

/// Next highest bit indicating
const NEXT_BIT: usize = 1 << (STATUS_BITS - 2);

/// A ABI transparent wrapper around EFI_STATUS
///
/// You should not need to use this directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EfiStatus(usize);

impl EfiStatus {
    ///
    #[inline]
    pub fn is_success(self) -> bool {
        self == EfiStatus::SUCCESS
    }

    ///
    #[inline]
    pub fn is_warning(self) -> bool {
        self.0 != 0 && self.0 & ERROR_BIT == 0
    }

    ///
    #[inline]
    pub fn is_error(self) -> bool {
        self.0 & ERROR_BIT != 0
    }

    ///
    #[inline]
    pub fn is_efi(self) -> bool {
        self.0 & ERROR_BIT != 0 && self.0 & NEXT_BIT == 0
    }

    ///
    #[inline]
    pub fn is_oem(self) -> bool {
        self.0 & NEXT_BIT != 0
    }
}

impl EfiStatus {
    pub const SUCCESS: Self = Self(0);

    pub const WARN_UNKNOWN_GLYPH: Self = Self(1);

    pub const INVALID_PARAMETER: Self = Self(ERROR_BIT | 2);

    pub const UNSUPPORTED: Self = Self(ERROR_BIT | 3);

    pub const DEVICE_ERROR: Self = Self(ERROR_BIT | 7);

    pub const ABORTED: Self = Self(ERROR_BIT | 21);

    pub const CRC_ERROR: Self = Self(ERROR_BIT | 27);
}

/// Represents a UEFI `EFI_STATUS`
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct UefiError {
    inner: EfiStatus,
}

impl UefiError {
    pub(crate) fn new(inner: EfiStatus) -> Self {
        Self { inner }
    }

    /// The [`EfiStatus`] for this error
    pub fn status(self) -> EfiStatus {
        self.inner
    }
}

impl From<EfiStatus> for Result<()> {
    fn from(value: EfiStatus) -> Self {
        if value.0 == 0 {
            Ok(())
        } else {
            Err(UefiError::new(value))
        }
    }
}

#[cfg(no)]
impl From<EfiStatus> for UefiError {
    fn from(value: EfiStatus) -> Self {
        UefiError::new(value)
    }
}

/// All [`core::fmt::Write`] failures are treated as [`EfiStatus::DEVICE_ERROR`]
impl From<core::fmt::Error> for UefiError {
    fn from(_: core::fmt::Error) -> Self {
        UefiError::new(EfiStatus::DEVICE_ERROR)
    }
}

impl core::fmt::Display for UefiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.status() {
            EfiStatus::SUCCESS => write!(f, "success"),
            EfiStatus::WARN_UNKNOWN_GLYPH => write!(f, "unknown glyph"),
            EfiStatus::INVALID_PARAMETER => write!(f, "invalid parameter"),
            EfiStatus::UNSUPPORTED => write!(f, "unsupported"),
            EfiStatus::DEVICE_ERROR => write!(f, "device error"),
            EfiStatus::ABORTED => write!(f, "aborted"),
            EfiStatus::CRC_ERROR => write!(f, "crc error"),
            status => write!(f, "{status:?}"),
        }
    }
}
