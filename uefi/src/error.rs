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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    pub const WARN_DELETE_FAILURE: Self = Self(2);

    pub const WARN_WRITE_FAILURE: Self = Self(3);

    pub const WARN_BUFFER_TOO_SMALL: Self = Self(4);

    pub const WARN_STALE_DATA: Self = Self(5);

    pub const WARN_FILE_SYSTEM: Self = Self(6);

    pub const WARN_RESET_REQUIRED: Self = Self(7);

    pub const LOAD_ERROR: Self = Self(ERROR_BIT | 1);

    pub const INVALID_PARAMETER: Self = Self(ERROR_BIT | 2);

    pub const UNSUPPORTED: Self = Self(ERROR_BIT | 3);

    pub const BAD_BUFFER_SIZE: Self = Self(ERROR_BIT | 4);

    pub const BUFFER_TOO_SMALL: Self = Self(ERROR_BIT | 5);

    pub const NOT_READY: Self = Self(ERROR_BIT | 6);

    pub const DEVICE_ERROR: Self = Self(ERROR_BIT | 7);

    pub const WRITE_PROTECTED: Self = Self(ERROR_BIT | 8);

    pub const OUT_OF_RESOURCES: Self = Self(ERROR_BIT | 9);

    pub const VOLUME_CORRUPTED: Self = Self(ERROR_BIT | 10);

    pub const VOLUME_FULL: Self = Self(ERROR_BIT | 11);

    pub const NO_MEDIA: Self = Self(ERROR_BIT | 12);

    pub const MEDIA_CHANGED: Self = Self(ERROR_BIT | 13);

    pub const NOT_FOUND: Self = Self(ERROR_BIT | 14);

    pub const ACCESS_DENIED: Self = Self(ERROR_BIT | 15);
    pub const NO_RESPONSE: Self = Self(ERROR_BIT | 16);
    pub const NO_MAPPING: Self = Self(ERROR_BIT | 17);
    pub const TIMEOUT: Self = Self(ERROR_BIT | 18);
    pub const NOT_STARTED: Self = Self(ERROR_BIT | 19);
    pub const ALREADY_STARTED: Self = Self(ERROR_BIT | 20);

    pub const ABORTED: Self = Self(ERROR_BIT | 21);

    pub const ICMP_ERROR: Self = Self(ERROR_BIT | 22);
    pub const TCP_ERROR: Self = Self(ERROR_BIT | 23);
    pub const PROTOCOL_ERROR: Self = Self(ERROR_BIT | 24);
    pub const INCOMPATIBLE_VERSION: Self = Self(ERROR_BIT | 25);
    pub const SECURITY_VIOLATION: Self = Self(ERROR_BIT | 26);

    pub const CRC_ERROR: Self = Self(ERROR_BIT | 27);

    pub const END_OF_MEDIA: Self = Self(ERROR_BIT | 28);
    pub const END_OF_FILE: Self = Self(ERROR_BIT | 31);
    pub const INVALID_LANGUAGE: Self = Self(ERROR_BIT | 32);
    pub const COMPROMISED_DATA: Self = Self(ERROR_BIT | 33);
    pub const IP_ADDRESS_CONFLICT: Self = Self(ERROR_BIT | 34);
    pub const HTTP_ERROR: Self = Self(ERROR_BIT | 35);
}

impl core::fmt::Display for EfiStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            //
            EfiStatus::SUCCESS => write!(f, "success"),

            // Warnings
            EfiStatus::WARN_UNKNOWN_GLYPH => write!(f, "unknown glyph"),
            EfiStatus::WARN_DELETE_FAILURE => write!(f, "delete failure"),
            EfiStatus::WARN_WRITE_FAILURE => write!(f, "write failure"),
            EfiStatus::WARN_BUFFER_TOO_SMALL => write!(f, "buffer too small warning"),
            EfiStatus::WARN_STALE_DATA => write!(f, "stale data"),
            EfiStatus::WARN_FILE_SYSTEM => write!(f, "filesystem"),
            EfiStatus::WARN_RESET_REQUIRED => write!(f, "reset required"),

            // Error
            EfiStatus::LOAD_ERROR => write!(f, "load error"),
            EfiStatus::INVALID_PARAMETER => write!(f, "invalid parameter"),
            EfiStatus::UNSUPPORTED => write!(f, "unsupported"),
            EfiStatus::BAD_BUFFER_SIZE => write!(f, "bad buffer"),
            EfiStatus::BUFFER_TOO_SMALL => write!(f, "buffer too small error"),
            EfiStatus::NOT_READY => write!(f, "not ready"),
            EfiStatus::DEVICE_ERROR => write!(f, "device error"),
            EfiStatus::WRITE_PROTECTED => write!(f, "write protected"),
            EfiStatus::OUT_OF_RESOURCES => write!(f, "out of resources"),
            EfiStatus::VOLUME_CORRUPTED => write!(f, "volume corrupted"),
            EfiStatus::VOLUME_FULL => write!(f, "volume full"),
            EfiStatus::NO_MEDIA => write!(f, "no media"),
            EfiStatus::MEDIA_CHANGED => write!(f, "media changed"),
            EfiStatus::NOT_FOUND => write!(f, "not found"),
            EfiStatus::ACCESS_DENIED => write!(f, "access denied"),
            EfiStatus::NO_RESPONSE => write!(f, "no response"),
            EfiStatus::NO_MAPPING => write!(f, "no mapping"),
            EfiStatus::TIMEOUT => write!(f, "time out"),
            EfiStatus::NOT_STARTED => write!(f, "not started"),
            EfiStatus::ALREADY_STARTED => write!(f, "already started"),
            EfiStatus::ABORTED => write!(f, "aborted"),
            EfiStatus::ICMP_ERROR => write!(f, "icmp error"),
            EfiStatus::TCP_ERROR => write!(f, "tcp error"),
            EfiStatus::PROTOCOL_ERROR => write!(f, "network protocol error"),
            EfiStatus::INCOMPATIBLE_VERSION => write!(f, "incompatible version"),
            EfiStatus::SECURITY_VIOLATION => write!(f, "security violation"),
            EfiStatus::CRC_ERROR => write!(f, "crc error"),
            EfiStatus::END_OF_MEDIA => write!(f, "end of media"),
            EfiStatus::END_OF_FILE => write!(f, "end of file"),
            EfiStatus::INVALID_LANGUAGE => write!(f, "invalid language"),
            EfiStatus::COMPROMISED_DATA => write!(f, "compromised data"),
            EfiStatus::IP_ADDRESS_CONFLICT => write!(f, "ip address conflict"),
            EfiStatus::HTTP_ERROR => write!(f, "http error"),
            // status => write!(f, "{status:?}"),
            _ => todo!(),
        }
    }
}

impl core::fmt::Debug for EfiStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("EfiStatus")
            .field(&self.0)
            .field(&format_args!("[Display] {}", self))
            .finish()
    }
}

/// Represents a UEFI `EFI_STATUS`
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct UefiError {
    inner: EfiStatus,
}

impl UefiError {
    /// Create a new [UefiError]
    ///
    /// Only do this with [EfiStatus] that is NOT [`EfiStatus::SUCCESS`]
    pub(crate) fn new(inner: EfiStatus) -> Self {
        debug_assert!(!inner.is_success(), "Tried to use UefiError with a success");
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
