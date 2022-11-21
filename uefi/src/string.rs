//! UEFI String handling helpers
//!
//! Note: This crate treats all UEFI strings as UTF-16
use alloc::{string::String, vec::Vec};
use core::{marker::PhantomData, slice::from_raw_parts};

use log::{error, trace};

use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    proto::{Guid, Protocol, Str16},
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
    /// Create an owned [UefiString] from `data` and `len` *characters*, NOT
    /// including nul.
    ///
    /// # Safety
    ///
    /// - Data must be a valid non-null pointer for `len` *characters*,
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
