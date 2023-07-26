//! UEFI String handling helpers
//!
//! Note: This crate treats all UEFI strings as UTF-16
use alloc::{string::String, vec::Vec};
use core::{
    char::REPLACEMENT_CHARACTER,
    fmt::Display,
    marker::PhantomData,
    mem::transmute,
    ops::Deref,
    slice::from_raw_parts,
};

use log::{error, trace};

use crate::{
    error::{Result, Status},
    get_boot_table,
    mem::MemoryType,
    proto::{
        device_path::{DevicePath, DevicePathToText, DevicePathUtil},
        Scope,
    },
    Boot,
    SystemTable,
};

fn to_ucs(s: &str) -> Vec<u16> {
    assert!(
        !s.contains('\0'),
        "Tried to use to_ucs with an internal null"
    );

    // Length in UTF-16, plus null.
    let cap = s.len() + 1;

    let mut out: Vec<u16> = Vec::with_capacity(cap);

    let mut write = out.spare_capacity_mut();

    // TODO: Actually UCS-2 instead of UTF-16?
    s.encode_utf16().chain([0]).zip(write).for_each(|(c, w)| {
        w.write(c);
    });

    // Safety:
    // - `out` should now be fully initialized for `cap`
    unsafe { out.set_len(cap) };

    // Just error if the input had any characters outside the UCS range
    if !s.chars().all(|c| (c as u32) < 0x10000) {
        panic!("invalid character in UcsString");
    }

    out
}

/// A UCS-2 string compatible with [`UefiString`],
/// but allocated using the Rust global allocator.
///
/// If the Global allocator and the UEFI allocator are the same,
/// these can safely be converted to [`UefiString`] at zero cost using the
/// TODO: unsafe functions [``]
#[derive(Debug)]
// #[repr(transparent)]
pub struct UcsString {
    data: Vec<u16>,
}

impl UcsString {
    pub fn new(s: &str) -> Self {
        Self { data: to_ucs(s) }
    }

    /// Get the string as a slice of u16 characters.
    ///
    /// Does not include trailing nul
    #[inline]
    pub fn as_slice(&self) -> &[u16] {
        &self.data[..self.data.len() - 1]
    }

    /// Get the string as a slice of u16 characters
    #[inline]
    pub fn as_slice_with_nul(&self) -> &[u16] {
        self.data.as_slice()
    }

    /// # Safety
    ///
    /// - `ptr` must contain a trailing nul
    /// - `ptr` must be valid for reads up to and including the null
    pub unsafe fn from_ptr(ptr: *const u16) -> Self {
        let len = string_len(ptr) + 1;
        Self::from_ptr_len(ptr, len)
    }

    /// # Safety
    ///
    /// [`Self::from_ptr`]
    pub unsafe fn from_ptr_len(data: *const u16, len: usize) -> Self {
        Self {
            data: from_raw_parts(data, len).to_vec(),
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u16 {
        self.data.as_ptr()
    }

    /// Unsafely get this [`UcsString`] as a [`UefiStr`]
    ///
    /// # Safety
    ///
    /// The global Rust allocator must be the UEFI pool allocator
    // TODO: Is this actually fine, would there be a problem with Uefi reading
    // strings from whatever memory we want?
    // So long as it doesn't think it owns it, it should be fine and safe?
    pub unsafe fn as_uefi_str(&self) -> UefiStr<'_> {
        UefiStr::from_ptr_len(self.data.as_ptr().cast_mut(), self.data.len())
    }
}

/// An owned UEFI string, encoded as UTF-16/UCS-2/lies*
///
/// *UEFI firmware supposedly often lies/is not conformant with UCS-2.
///
/// The backing memory will be freed using
/// [`crate::table::BootServices::free_pool`] on [Drop]
///
/// This means this data is only valid before ExitBootServices.
#[derive(Debug)]
#[repr(C)]
pub struct UefiString<'table> {
    data: *mut u16,

    /// Length in *characters*, including null.
    len: usize,

    /// Lifetime erased UefiStr to ourselves
    ///
    /// This is a hack used for [`Deref::deref`] and [`AsRef::as_ref`], which
    /// return *references*, when we dont have a DST.
    ref_: UefiStr<'table>,

    _ghost: PhantomData<&'table u8>,
}

impl<'table> UefiString<'table> {
    /// Create a new, null terminated, UEFI string
    ///
    /// # Panics
    ///
    /// - If `s` has any internal nulls
    /// - Failure to allocate memory
    #[track_caller]
    pub fn new(s: &str) -> Self {
        assert!(
            !s.contains('\0'),
            "Tried to create a UefiString with an internal null"
        );
        let table = get_boot_table().unwrap();
        let boot = table.boot();
        // Length in UTF-16, plus null.
        let cap = s.len() + 1;

        // Safety: aligned
        let data = unsafe {
            boot.allocate_pool_ty_array::<u16>(MemoryType::LOADER_DATA, cap)
                .unwrap()
                .cast::<u16>()
        };
        let mut write = data.as_ptr();

        s.encode_utf16().chain([0]).for_each(|c| {
            // Safety: `write` is valid for `cap`,
            // which is the length of `s` and nul terminator.
            unsafe {
                write.write(c);
                write = write.add(1);
            }
        });
        // `data` should now be fully initialized

        let len = cap;
        let data = data.as_ptr();
        Self {
            data,
            len,
            ref_: UefiStr {
                data,
                len,
                _ghost: PhantomData,
            },
            _ghost: PhantomData,
        }
    }

    /// Create an owned [UefiString] from `data`
    ///
    /// This takes responsibility for freeing the memory using `free_pool`
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in null
    pub unsafe fn from_ptr(data: *mut u16) -> Self {
        let len = string_len(data) + 1;
        Self::from_ptr_len(data, len)
    }

    /// Create an owned [UefiString] from `data` and `len` *characters*,
    /// including nul.
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in nul
    pub unsafe fn from_ptr_len(data: *mut u16, len: usize) -> Self {
        Self {
            data,
            len,
            ref_: UefiStr {
                data,
                len,
                _ghost: PhantomData,
            },
            _ghost: PhantomData,
        }
    }
}

impl<'table> AsRef<UefiStr<'table>> for UefiString<'table> {
    fn as_ref(&self) -> &UefiStr<'table> {
        &self.ref_
    }
}

impl<'table> From<&str> for UefiString<'table> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'table> Deref for UefiString<'table> {
    type Target = UefiStr<'table>;

    fn deref(&self) -> &Self::Target {
        &self.ref_
    }
}

impl<'table> Drop for UefiString<'table> {
    fn drop(&mut self) {
        if let Some(table) = get_boot_table() {
            // Safety: self.data was allocated by allocate_pool
            let _ = unsafe { table.boot().free_pool(self.data.cast()) };
        }
    }
}

impl<'table> Display for UefiString<'table> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.ref_.fmt(f)
    }
}

/// An unowned UEFI string.
///
/// See [`UefiString`] for more details.
// This type is not unsized, and yet still should ONLY be created behind a reference.
// Specifically, a reference to the owning [`UefiString`]
// This is depended on for safety internally, to prevent UAF.
#[derive(Debug)]
#[repr(C)]
pub struct UefiStr<'buf> {
    data: *mut u16,

    /// Length in *characters*, including null.
    len: usize,

    _ghost: PhantomData<&'buf u16>,
}

impl<'buf> UefiStr<'buf> {
    /// Create an unowned [`UefiStr`] from `data`
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in nul
    pub unsafe fn from_ptr(data: *mut u16) -> Self {
        let len = string_len(data) + 1;
        Self::from_ptr_len(data, len)
    }

    /// Create an unowned [`UefiStr`] from `data` and `len` *characters*,
    /// including nul.
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in nul
    pub unsafe fn from_ptr_len(data: *mut u16, len: usize) -> Self {
        Self {
            data,
            len,
            _ghost: PhantomData,
        }
    }

    /// Get the pointer to the [`UefiStr`]s buffer
    #[inline]
    pub const fn as_ptr(&self) -> *const u16 {
        self.data
    }

    /// Get the string as a slice of u16 characters.
    ///
    /// Does not include trailing nul
    #[inline]
    pub const fn as_slice(&self) -> &[u16] {
        // Safety: Ensured valid in from_ptr
        unsafe { from_raw_parts(self.data, self.len - 1) }
    }

    /// Get the string as a slice of u16 characters
    #[inline]
    pub const fn as_slice_with_nul(&self) -> &[u16] {
        // Safety: Ensured valid in from_ptr
        unsafe { from_raw_parts(self.data, self.len) }
    }

    /// Convert the [`UefiString`] into a [`String`], replacing invalid
    /// characters
    pub fn to_string_lossy(&self) -> String {
        char::decode_utf16(self.as_slice().iter().copied())
            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            .collect::<String>()
    }
}

impl<'buf> Display for UefiStr<'buf> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_string_lossy())
    }
}

/// An unowned UEFI [DevicePath]
#[derive(Debug)]
pub struct Path<'table> {
    data: DevicePath<'table>,
}

impl<'table> Path<'table> {
    /// Create an unowned [Path] from a [DevicePath]
    pub fn new(data: DevicePath<'table>) -> Self {
        Self { data }
    }

    /// Convert [`Path`] to [`PathBuf`]
    pub fn to_path_buf(&self) -> Result<PathBuf<'table>> {
        let copy = self.data.duplicate()?;
        let v = PathBuf::new(copy);
        Ok(v)
    }

    /// Convert this path to a UEFI String
    pub fn to_text(&'table self) -> Result<UefiString<'table>> {
        self.data.to_uefi_string()
    }

    /// Convert this path to a Rust String
    ///
    /// Invalid characters are mapped to [`char::REPLACEMENT_CHARACTER`]
    pub fn to_string_lossy(&self) -> Result<String> {
        let s = self.data.to_uefi_string()?;
        let s = s.as_slice();
        let s = char::decode_utf16(s.iter().copied())
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>();
        Ok(s)
    }

    /// Get this as a [DevicePath]
    pub fn as_device(&self) -> &DevicePath<'table> {
        &self.data
    }
}

impl<'table> Display for Path<'table> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Ok(s) = self.to_string_lossy() {
            write!(f, "{s}")
        } else {
            write!(f, "Path (couldn't display, out of memory)")
        }
    }
}

/// An owned UEFI [DevicePath]
#[derive(Debug)]
pub struct PathBuf<'table> {
    data: DevicePath<'table>,
}

impl<'table> PathBuf<'table> {
    pub(crate) fn new(data: DevicePath<'table>) -> Self {
        Self { data }
    }

    /// Pop the last component off from the [Path]
    pub fn pop(&self) -> Result<PathBuf> {
        let copy = self.try_clone()?;
        // TODO: Figure out how to manipulate DevicePaths

        todo!()
    }

    pub fn as_path(&self) -> Path {
        // Safety: `self.data` is valid
        unsafe { Path::new(DevicePath::new(self.data.as_ptr())) }
    }

    pub fn try_clone(&self) -> Result<Self> {
        Ok(PathBuf::new(self.data.duplicate()?))
    }
}

impl<'table> Clone for PathBuf<'table> {
    #[inline]
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

impl<'table> Drop for PathBuf<'table> {
    fn drop(&mut self) {
        if let Some(table) = get_boot_table() {
            // Safety: We own this path
            let _ = unsafe { self.data.free(&table.boot()) };
        }
    }
}

/// Get the length of a string, not including null.
///
/// # Safety
///
/// - data must be a valid non-null pointer to a string
#[inline]
pub(crate) const unsafe fn string_len(data: *const u16) -> usize {
    let mut len = 0;
    while *data.add(len) != 0 {
        len += 1;
    }
    len
}
