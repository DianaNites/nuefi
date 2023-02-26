//! UEFI String handling helpers
//!
//! Note: This crate treats all UEFI strings as UTF-16
use alloc::string::String;
use core::{fmt::Display, marker::PhantomData, ops::Deref, slice::from_raw_parts};

use log::{error, trace};

use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    proto::device_path::{DevicePath, DevicePathToText},
    Boot,
    SystemTable,
};

fn table() -> Result<SystemTable<Boot>> {
    if let Some(table) = get_boot_table() {
        Ok(table)
    } else {
        error!("Tried to use `Path` while not in `Boot` mode");
        Err(UefiError::new(EfiStatus::UNSUPPORTED))
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

    /// Length in *characters*
    len: usize,

    _ghost: PhantomData<&'table mut u8>,
}

impl<'table> UefiString<'table> {
    /// Create an owned [UefiString] from `data`
    ///
    /// This takes responsibility for freeing the memory using `free_pool`
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in nul
    pub unsafe fn from_ptr(data: *mut u16) -> Self {
        Self {
            data,
            len: string_len(data) + 1,
            _ghost: PhantomData,
        }
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
            _ghost: PhantomData,
        }
    }
}

impl<'table> Deref for UefiString<'table> {
    type Target = UefiStr<'table>;

    fn deref(&self) -> &Self::Target {
        // Safety: UefiStr and UefiString have the same layout
        unsafe { core::mem::transmute(self) }
    }
}

impl<'table> Drop for UefiString<'table> {
    fn drop(&mut self) {
        trace!("Deallocating UefiString");
        if let Some(table) = get_boot_table() {
            // Safety: self.data was allocated by UEFI
            let ret = unsafe { table.boot().free_pool(self.data as *mut u8) };
            if ret.is_err() {
                error!("Failed to deallocate UefiString {:p}", self.data)
            }
        } else {
            error!(
                "Tried to deallocate UefiString {:p} while not in Boot mode",
                self.data
            )
        }
    }
}

/// An unowned UEFI string.
///
/// See [UefiString] for more details.
#[derive(Debug)]
#[repr(C)]
pub struct UefiStr<'table> {
    data: *mut u16,

    /// Length in *characters*
    len: usize,

    _ghost: PhantomData<&'table mut u8>,
}

impl<'table> UefiStr<'table> {
    /// Create an unowned [UefiStr] from `data`
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer to a UEFI string ending in nul
    pub unsafe fn from_ptr(data: *mut u16) -> Self {
        Self {
            data,
            len: string_len(data) + 1,
            _ghost: PhantomData,
        }
    }

    /// Create an unowned [UefiStr] from `data` and `len` *characters*,
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

    /// Get the string as a slice of u16 characters.
    ///
    /// Does not include trailing nul
    pub const fn as_slice(&self) -> &[u16] {
        // Safety: Ensured valid in from_ptr
        unsafe { from_raw_parts(self.data, self.len - 1) }
    }

    /// Get the string as a slice of u16 characters
    pub const fn as_slice_with_nul(&self) -> &[u16] {
        // Safety: Ensured valid in from_ptr
        unsafe { from_raw_parts(self.data, self.len) }
    }

    /// Convert this to a string
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        char::decode_utf16(self.as_slice().iter().cloned())
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>()
    }
}

/// An unowned UEFI [DevicePath]
#[derive(Debug)]
pub struct Path<'table> {
    data: DevicePath<'table>,
}

impl<'table> Path<'table> {
    /// Create an unowned [Path] from a [DevicePath]
    pub(crate) fn new(data: DevicePath<'table>) -> Self {
        Self { data }
    }

    /// Convert this path to a UEFI String
    pub fn to_text(&'table self) -> Result<UefiString<'table>> {
        let table = table()?;
        let boot = table.boot();
        let text = boot
            .locate_protocol::<DevicePathToText>()?
            .ok_or_else(|| UefiError::new(EfiStatus::UNSUPPORTED))?;

        let s = text.convert_device_path_to_text(&self.data)?;
        Ok(
            // Safety: Evil lifetime hack, turn our local borrow
            // into a `'table` borrow
            // This should be safe because the only way to call to_text is
            // by having a valid lifetime
            unsafe { core::mem::transmute(s) },
        )
    }

    /// Convert this path to a Rust String
    ///
    /// Invalid characters are mapped to [`char::REPLACEMENT_CHARACTER`]
    pub fn to_string(&self) -> Result<String> {
        let table = table()?;
        let boot = table.boot();
        let text = boot
            .locate_protocol::<DevicePathToText>()?
            .ok_or_else(|| UefiError::new(EfiStatus::UNSUPPORTED))?;

        let s = text.convert_device_path_to_text(&self.data)?;
        let s = s.as_slice();
        let s = char::decode_utf16(s.iter().cloned())
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
        if let Ok(s) = self.to_string() {
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

impl<'table> Drop for PathBuf<'table> {
    fn drop(&mut self) {
        trace!("Deallocating DevicePath");
        if let Some(table) = get_boot_table() {
            let ret = self.data.free(&table.boot());
            if ret.is_err() {
                error!("Failed to deallocate DevicePath {:?}", self.data)
            }
        } else {
            error!(
                "Tried to deallocate DevicePath {:?} while not in Boot mode",
                self
            )
        }
    }
}

/// Get the length of a string
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
