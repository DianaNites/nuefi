//! UEFI Base types
//!
//! # Core Types
//!
//! UEFI Defines several core types which have preferred native Rust
//! equivalents. They are:
//!
//! - `INTN`/`UINTN` = `isize`/`usize`
//! - `UINT<X>` = `u<X>`, where `X` = `8`, `16`, `32`, `64`, `128`
//! - `INT<X>` = `i<X>`, where `X` = `8`, `16`, `32`, `64`, `128`
//! - `VOID` = [`c_void`][`core::ffi::c_void`]
//!
//! See [uefi_dt] for more details
//!
//! # References
//!
//! - [UEFI Section 2.3.][uefi_cc]
//!
//! [uefi_cc]: <https://uefi.org/specs/UEFI/2.10/02_Overview.html#calling-conventions>
//! [uefi_dt]: <https://uefi.org/specs/UEFI/2.10/02_Overview.html#common-uefi-data-types>
use core::{ffi::c_void, fmt, ptr::null_mut};

use nuuid::Uuid;

/// Bits in [`Status`]
// Just to make it less annoying if we end up supporting 128-bit platforms,
// because iirc rust's usize wont be 128-bit there?
const STATUS_BITS: u32 = usize::BITS;

/// High bit indicating error
const ERROR_BIT: usize = 1 << (STATUS_BITS - 1);

/// Next highest bit indicating
const NEXT_BIT: usize = 1 << (STATUS_BITS - 2);

/// UEFI logical Boolean type
///
/// This is ABI Identical to a `u8`, but maps `0` to [`false`]
/// and non-zero to [`true`].
///
/// We provide this because while UEFI does define their `BOOLEAN`
/// to be either `0`, `1`, or undefined, apparently in the wild
/// many implementations accept any non-zero valid, and
/// either way on the Rust side, we must be defensive.
///
/// If UEFI ever gives us an invalid [`bool`], that would be
/// immediate Rust UB, whereas this type is valid for all `u8`.
/// This type ensures we are *always* sound.
///
/// Despite this, this type is still treated as a bool.
#[derive(Debug, Clone, Copy, Eq, Default)]
#[repr(transparent)]
pub struct Boolean(u8);

impl Boolean {
    #[inline]
    pub const fn to_bool(self) -> bool {
        self.0 != 0
    }
}

impl Ord for Boolean {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.to_bool().cmp(&other.to_bool())
    }
}

impl PartialOrd for Boolean {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.to_bool().partial_cmp(&other.to_bool())
    }
}

impl PartialEq for Boolean {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.to_bool().eq(&other.to_bool())
    }
}

impl From<bool> for Boolean {
    #[inline]
    fn from(value: bool) -> Self {
        Self(value as u8)
    }
}

impl From<Boolean> for bool {
    #[inline]
    fn from(value: Boolean) -> Self {
        value.to_bool()
    }
}

impl fmt::Display for Boolean {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_bool().fmt(f)
    }
}

/// A 1-byte UEFI character, ASCII Latin-1 unless specified otherwise.
pub type Char8 = u8;

/// A 2-byte UEFI character, UCS-2/UTF-16 Latin-1 as defined by the Unicode
/// 2.1 and ISO/IEC 10646 standards unless specified otherwise.
pub type Char16 = u16;

/// UEFI Globally Unique Identifier, or GUID.
///
/// This is FFI compatible with and ABI Identical to a 128-bit buffer thats
/// 64-bit aligned, aka a suitably aligned `[u8; 16]`, or a `u128`.
///
/// A GUID is a Microsoft Format [RFC 4122 UUID],
/// with these caveats from [Appendix A. GUID and Time Formats][aa].
///
/// It is important to read that document to understand the layout of
/// this buffer, if using it directly. UEFI relies extensively on GUIDs.
///
/// [rfc4122]: <https://www.rfc-editor.org/rfc/rfc4122>
/// [aa]: <https://uefi.org/specs/UEFI/2.10/Apx_A_GUID_and_Time_Formats.html>
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Guid([u8; 16]);

impl Guid {
    /// Create a new [`Guid`] directly from `bytes`
    #[inline]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Raw, *unaligned*, GUID bytes
    #[inline]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    // TODO: Replace with `new`
    #[inline]
    #[doc(hidden)]
    // #[deprecated(note = "Nuuid use new")]
    pub const unsafe fn from_bytes(bytes: [u8; 16]) -> Self {
        // FIXME: Uhh.. why? This is wrong. The proc macro should be doing this.
        Self(nuuid::Uuid::from_bytes_me(bytes).to_bytes())
        // Self::new(bytes)
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let uuid = Uuid::from_bytes_me(self.0);
        f.debug_tuple("Guid") //.
            .field(&self.0)
            .field(&format_args!("[Guid] {uuid}"))
            .finish()
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let uuid = Uuid::from_bytes_me(self.0);
        uuid.fmt(f)
    }
}

/// UEFI Status codes
///
/// This is FFI compatible with and ABI Identical to a [`usize`]
///
/// # References
///
/// See [Appendix D. Status Codes][ad] for exact details on status values
///
/// [ad]: <https://uefi.org/specs/UEFI/2.10/Apx_D_Status_Codes.html>
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(usize);

impl Status {
    /// Create a new [`Status`]
    #[inline]
    pub const fn new(code: usize) -> Self {
        Self(code)
    }

    /// Raw UEFI status code
    #[inline]
    pub const fn code(self) -> usize {
        self.0
    }

    /// Returns whether this status represents success
    #[inline]
    pub const fn is_success(self) -> bool {
        self.0 == Self::SUCCESS.0
    }

    /// Returns whether this status represents a warning
    #[inline]
    pub const fn is_warning(self) -> bool {
        self.0 != 0 && self.0 & ERROR_BIT == 0
    }

    /// Returns whether this status represents a error
    #[inline]
    pub const fn is_error(self) -> bool {
        self.0 & ERROR_BIT != 0
    }

    /// Returns whether this status is reserved for use by UEFI
    #[inline]
    pub const fn is_efi(self) -> bool {
        self.0 & ERROR_BIT != 0 && self.0 & NEXT_BIT == 0
    }

    /// Returns whether this status is reserved for use by OEMs
    #[inline]
    pub const fn is_oem(self) -> bool {
        self.0 & NEXT_BIT != 0
    }
}

impl Status {
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

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            //
            Status::SUCCESS => write!(f, "success"),

            // Warnings
            Status::WARN_UNKNOWN_GLYPH => write!(f, "unknown glyph"),
            Status::WARN_DELETE_FAILURE => write!(f, "delete failure"),
            Status::WARN_WRITE_FAILURE => write!(f, "write failure"),
            Status::WARN_BUFFER_TOO_SMALL => write!(f, "buffer too small warning"),
            Status::WARN_STALE_DATA => write!(f, "stale data"),
            Status::WARN_FILE_SYSTEM => write!(f, "filesystem"),
            Status::WARN_RESET_REQUIRED => write!(f, "reset required"),

            // Error
            Status::LOAD_ERROR => write!(f, "load error"),
            Status::INVALID_PARAMETER => write!(f, "invalid parameter"),
            Status::UNSUPPORTED => write!(f, "unsupported"),
            Status::BAD_BUFFER_SIZE => write!(f, "bad buffer"),
            Status::BUFFER_TOO_SMALL => write!(f, "buffer too small error"),
            Status::NOT_READY => write!(f, "not ready"),
            Status::DEVICE_ERROR => write!(f, "device error"),
            Status::WRITE_PROTECTED => write!(f, "write protected"),
            Status::OUT_OF_RESOURCES => write!(f, "out of resources"),
            Status::VOLUME_CORRUPTED => write!(f, "volume corrupted"),
            Status::VOLUME_FULL => write!(f, "volume full"),
            Status::NO_MEDIA => write!(f, "no media"),
            Status::MEDIA_CHANGED => write!(f, "media changed"),
            Status::NOT_FOUND => write!(f, "not found"),
            Status::ACCESS_DENIED => write!(f, "access denied"),
            Status::NO_RESPONSE => write!(f, "no response"),
            Status::NO_MAPPING => write!(f, "no mapping"),
            Status::TIMEOUT => write!(f, "time out"),
            Status::NOT_STARTED => write!(f, "not started"),
            Status::ALREADY_STARTED => write!(f, "already started"),
            Status::ABORTED => write!(f, "aborted"),
            Status::ICMP_ERROR => write!(f, "icmp error"),
            Status::TCP_ERROR => write!(f, "tcp error"),
            Status::PROTOCOL_ERROR => write!(f, "network protocol error"),
            Status::INCOMPATIBLE_VERSION => write!(f, "incompatible version"),
            Status::SECURITY_VIOLATION => write!(f, "security violation"),
            Status::CRC_ERROR => write!(f, "crc error"),
            Status::END_OF_MEDIA => write!(f, "end of media"),
            Status::END_OF_FILE => write!(f, "end of file"),
            Status::INVALID_LANGUAGE => write!(f, "invalid language"),
            Status::COMPROMISED_DATA => write!(f, "compromised data"),
            Status::IP_ADDRESS_CONFLICT => write!(f, "ip address conflict"),
            Status::HTTP_ERROR => write!(f, "http error"),
            status => write!(f, "{status:?}"),
            // _ => unimplemented!(),
        }
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Status")
            .field(&self.0)
            .field(&format_args!("[Display] {self}"))
            .finish()
    }
}

/// An opaque handle to a UEFI object
///
/// This is FFI compatible with and ABI Identical to a
/// [`*mut c_void`], and may be null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Handle(*mut c_void);

impl Handle {
    /// Create a new [`Handle`]
    ///
    /// # Safety
    ///
    /// By calling this, you assert that `p` actually does
    /// point to a legitimate UEFI handle.
    ///
    /// There is no reason you should ever need this,
    /// as a UEFI application.
    ///
    /// All UEFI handles are assumed to be.. *UEFI handles*.
    /// Their implementation is undefined,
    /// but they must be some common structure so they can be
    /// properly identified by the various functions that take this
    /// or safely return an error on an invalid handle.
    /// At least, that would be a valid implementation.
    ///
    /// Whatever random value you pass wont be.
    ///
    /// This is a massive safety invariant relied on throughout
    /// the library.
    #[inline]
    pub const unsafe fn new(p: *mut c_void) -> Self {
        Self(p)
    }

    /// Create a new null [`Handle`]
    ///
    /// This is safe because a null [`Handle`] is an error, and
    /// we maintain this invariant where needed.
    #[inline]
    pub const fn null() -> Self {
        Self(null_mut())
    }

    /// Get the pointer for this [`Handle`]
    #[inline]
    pub const fn as_ptr(self) -> *mut c_void {
        self.0
    }
}

/// An opaque handle to a UEFI event
///
/// This is FFI compatible with and ABI Identical to a
/// [`*mut c_void`], and may be null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Event(*mut c_void);

impl Event {
    /// Get the pointer for this [`Event`]
    #[inline]
    pub const fn as_ptr(self) -> *mut c_void {
        self.0
    }
}

/// UEFI Logical Block Address, or LBA.
///
/// This is FFI compatible with and ABI Identical to a [`u64`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LogicalBlockAddress(u64);

/// Task Priority Level
///
/// This is FFI compatible with and ABI Identical to a [`usize`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TaskPriorityLevel(usize);

/// 32-byte buffer containing a MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct MacAddress([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct IPV4([u8; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct IPV6([u8; 16]);

/// An [`IPV4`] or [`IPV6`] address
///
/// A 16-byte buffer aligned on 4 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C, align(4))]
pub struct IP([u8; 16]);

impl IP {
    /// Get a pointer to the aligned buffer
    #[inline]
    pub const fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    /// Get a mutable pointer to the aligned buffer
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}
