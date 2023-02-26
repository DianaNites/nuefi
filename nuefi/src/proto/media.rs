//! UEFI Media protocols
use alloc::vec::Vec;
use core::{iter::once, ptr::null_mut};

use raw::*;

use crate::{
    error::{Result, UefiError},
    proto::{Guid, Protocol},
    util::interface,
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("4006C0C1-FCB3-403E-996D-4A6C8724E06D", crate = "crate")]
    LoadFile2(RawLoadFile2)
);

impl<'table> LoadFile2<'table> {
    //
}

interface!(
    /// UEFI Simple filesystem protocol.
    /// Gives [`File`] based access to a device.
    ///
    /// UEFI supports the `FAT{12,16,32}` filesystems out of the box
    #[Protocol("0964E5B02-2645-911D-28E3-900A0C969723B", crate = "crate")]
    SimpleFileSystem(RawSimpleFileSystem)
);

impl<'table> SimpleFileSystem<'table> {
    /// Open the root directory of a volume
    ///
    /// It is your responsibility to call [`File::close`].
    /// If you don't, the volume will remain open.
    pub fn open_volume(&self) -> Result<File> {
        let mut out = null_mut();
        // Safety: `file` is always valid, checked for null
        // anything else is the responsibility of firmware
        let ret = unsafe { (self.interface().open_volume.unwrap())(self.interface, &mut out) };
        if ret.is_success() {
            assert!(
                !out.is_null(),
                "SimpleFileSystem returned success, but the file was null.",
            );
            // Safety: `File` isn't a Protocol
            unsafe { Ok(File::new(out)) }
        } else {
            Err(UefiError::new(ret))
        }
    }
}

interface!(
    /// UEFI File Protocol
    ///
    /// This represents both files and directories on a filesystem.
    ///
    /// # Note
    ///
    /// This does not have a [`trait@Protocol`] implementation because this
    /// is not a standalone protocol.
    ///
    /// See [`SimpleFileSystem`]
    // TODO: Is File a good name? its more like a path but like.. not a path?
    File(RawFile)
);

impl<'table> File<'table> {
    fn open_impl(&self, name: &str, mode: u64, flags: u64) -> Result<File> {
        let mut out = null_mut();
        let name: Vec<u16> = name.encode_utf16().chain(once(0)).collect();

        // Safety: `out` valid by definition, firmware
        let ret = unsafe {
            (self.interface().open.unwrap())(self.interface, &mut out, name.as_ptr(), mode, flags)
        };

        if ret.is_success() {
            assert!(
                !out.is_null(),
                "File returned success, but the file was null.",
            );
            // Safety: `File` isn't a Protocol
            unsafe { Ok(File::new(out)) }
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Open a new `File` relative to this one
    pub fn open(&self, name: &str) -> Result<File> {
        let mode = 0x1;
        let flags = 0;
        self.open_impl(name, mode, flags)
    }

    /// Create a new `File` relative to this one
    pub fn create(&self, name: &str) -> Result<File> {
        todo!()
    }

    /// Close the handle, flushing all data, waiting for any pending async I/O.
    pub fn close(self) -> Result<()> {
        // Safety: checked for null, anything else is the responsibility of firmware
        unsafe { (self.interface().close.unwrap())(self.interface) }.into()
    }
}
