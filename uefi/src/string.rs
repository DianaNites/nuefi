//! UEFI String handling helpers
//!
//! Note: This crate treats all UEFI strings as UTF-16
use alloc::{string::String, vec::Vec};
use core::{marker::PhantomData, slice::from_raw_parts};

use log::{error, trace};

use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    proto::{
        device_path::{DevicePath, DevicePathToText},
        Guid,
        Protocol,
        Str16,
    },
    util::interface,
};

/// An owned UEFI string, encoded as UTF-16/UCS-2/lies*
///
/// *UEFI firmware supposedly often lies/is not conformant with UCS-2.
///
/// The backing memory will be freed using [`crate::SystemTable::free_pool`] on
/// [Drop]
///
/// This means this data is only valid before ExitBootServices.
#[derive(Debug)]
pub struct UefiString<'table> {
    data: *mut u16,

    /// Length in *characters*
    len: usize,

    _ghost: PhantomData<&'table mut u8>,
}

impl<'table> UefiString<'table> {
    /// Create an owned [UefiString] from `data` and `len` *characters*.
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer for `len` *characters*, not
    ///   including nul
    pub(crate) unsafe fn from_raw(data: *mut u16, len: usize) -> Self {
        Self {
            data,
            len,
            _ghost: PhantomData,
        }
    }

    pub(crate) fn as_slice(&self) -> &[u16] {
        unsafe { from_raw_parts(self.data, self.len) }
    }
}

impl<'table> Drop for UefiString<'table> {
    fn drop(&mut self) {
        trace!("Deallocating UefiString");
        if let Some(table) = get_boot_table() {
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

///
pub struct UefiStr {
    data: *mut u16,
}

impl UefiStr {
    pub(crate) fn from_raw(data: *mut u16) -> Self {
        Self { data }
    }
}

/// An unowned UEFI [DevicePath]
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
        if let Some(table) = get_boot_table() {
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
        } else {
            error!("Tried to use DevicePath::to_text while not in Boot mode");
            Err(UefiError::new(EfiStatus::UNSUPPORTED))
        }
    }

    /// Convert this path to a Rust String
    ///
    /// Invalid characters are mapped to [`char::REPLACEMENT_CHARACTER`]
    pub fn to_string(&self) -> Result<String> {
        if let Some(table) = get_boot_table() {
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
        } else {
            error!("Tried to use DevicePath::to_string while not in Boot mode");
            Err(UefiError::new(EfiStatus::UNSUPPORTED))
        }
    }
}

/// An owned UEFI [DevicePath]
pub struct PathBuf {
    //
}

#[cfg(no)]
impl<'table> Drop for PathBuf<'table> {
    fn drop(&mut self) {
        trace!("Deallocating DevicePath");
        if let Some(table) = get_boot_table() {
            let ret = unsafe { table.boot().free_pool(self.interface as *mut u8) };
            if ret.is_err() {
                error!("Failed to deallocate DevicePath {:p}", self.interface)
            }
        } else {
            error!(
                "Tried to deallocate DevicePath {:p} while not in Boot mode",
                self.interface
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
pub(crate) unsafe fn string_len(data: *mut u16) -> usize {
    let mut len = 0;
    while *data.add(len) != 0 {
        len += 1;
    }
    len
}
