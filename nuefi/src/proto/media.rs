//! UEFI Media protocols
use alloc::{string::String, vec::Vec};
use core::{
    iter::once,
    mem::{size_of, MaybeUninit},
    ptr::null_mut,
    slice::from_raw_parts,
};

use log::trace;
use raw::*;

use crate::{
    error::{EfiStatus, Result, UefiError},
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
    #[Protocol("964E5B22-6459-11D2-8E39-00A0C969723B", crate = "crate")]
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

    fn read_impl(&self, dir: bool) -> Result<Vec<u8>> {
        let mut size = 0;
        let mut out: Vec<u8> = Vec::new();

        // Safety: Described within
        unsafe {
            let rd = self.interface().read.unwrap();

            // Calling to get buffer size

            // `interface` and `size` are always valid
            // ptr is null and thats okay
            let ret = (rd)(self.interface, &mut size, null_mut());
            if dir && size == 0 && ret.is_success() {
                // End of directories
                return Ok(Vec::new());
            } else if ret != EfiStatus::BUFFER_TOO_SMALL {
                // Anything other than `BUFFER_TOO_SMALL` here is an error
                return Err(UefiError::new(ret));
            }

            // Here we reserve enough memory for `size`, initializing to `0`.
            out.resize(size, 0);

            // Assert just in case?
            assert!(out.capacity() >= size, "File read capacity bug");
            let ptr = out.as_mut_ptr() as *mut u8;

            // Calling to fill the buffer

            // `interface`, `size`, are valid
            // `ptr` is valid for `size` bytes
            let ret = (rd)(self.interface, &mut size, ptr);

            if ret.is_success() {
                // We only call this on success, and before returning.
                // Out has been fully initialized, because we started initialized
                out.set_len(size);

                Ok(out)
            } else {
                Err(ret.into())
            }
        }
    }

    /// Read the contents of the directory referred to by our handle
    ///
    /// This skips the `.` and `..` entries
    pub fn read_dir(&self) -> Result<impl Iterator<Item = Result<FileInfo>> + '_> {
        let info = self.info()?;
        if !info.directory() {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }

        let mut stop = false;

        // TODO: Clone impl?
        let me = self.open(".")?;

        Ok(core::iter::from_fn(move || loop {
            if stop {
                return None;
            }
            let ret = match me.read_impl(true) {
                Ok(v) => {
                    // Signals EOF
                    if v.is_empty() {
                        stop = true;
                        return None;
                    }
                    let info = FileInfo::from_bytes(v).unwrap();
                    let name = info.name();
                    if name == "." || name == ".." {
                        continue;
                    }
                    Some(Ok(info))
                }
                Err(e) => Some(Err(e)),
            };
            break ret;
        }))
    }

    /// Information about this [`File`]. See [`FileInfo`]
    pub fn info(&self) -> Result<FileInfo> {
        // FIXME: GUID macro 09576E92-6D3F-11D2-8E39-00A0C969723B
        #[allow(clippy::undocumented_unsafe_blocks)]
        const GUID: Guid = unsafe {
            Guid::from_bytes([
                0x09, 0x57, 0x6e, 0x92, 0x6d, 0x3f, 0x11, 0xd2, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69,
                0x72, 0x3b,
            ])
        };
        let guid = GUID;
        let mut size: usize = 0;
        let mut out: Vec<u8> = Vec::new();

        // Safety: Described within
        unsafe {
            let size_ptr: *mut usize = &mut size;

            let fp = self.interface().get_info.unwrap();

            // Get the buffer size in `size`
            let info = (fp)(self.interface, &guid, size_ptr, null_mut());

            // It should be `BUFFER_TOO_SMALL`
            if info != EfiStatus::BUFFER_TOO_SMALL {
                return Err(UefiError::new(info));
            }
            // Sanity check
            if size == 0 {
                return Err(UefiError::new(EfiStatus::INVALID_PARAMETER));
            }
            out.reserve_exact(size);
            assert!(out.capacity() >= size, "FileInfo capacity bug");
            // `ptr` was invalidated
            let ptr = out.as_mut_ptr();

            // This time fill buffer
            let info = (fp)(self.interface, &guid, &mut size, ptr);

            if info.is_success() {
                // Set `out`'s length
                out.set_len(size);
                let info = FileInfo::from_bytes(out.clone()).unwrap();
                Ok(info)
            } else {
                Err(UefiError::new(info))
            }
        }
    }

    /// Close the handle, flushing all data, waiting for any pending async I/O.
    pub fn close(self) -> Result<()> {
        // Safety: checked for null, anything else is the responsibility of firmware
        unsafe { (self.interface().close.unwrap())(self.interface) }.into()
    }
}

/// UEFI [`File`] information
///
/// This represents both files and directories on a filesystem.
// TODO: Separate GUID and Protocol traits?
#[derive(Debug)]
pub struct FileInfo {
    info: RawFileInfo,
    name: String,
}

impl FileInfo {
    const DIRECTORY: u64 = 0x10;

    fn new(info: RawFileInfo, name: String) -> Self {
        Self { info, name }
    }

    /// Create `FileInfo` from bytes
    fn from_bytes(v: Vec<u8>) -> Result<FileInfo> {
        // Safety: Described within
        unsafe {
            let mut info: MaybeUninit<RawFileInfo> = MaybeUninit::uninit();
            let f_size = size_of::<RawFileInfo>();

            // Split off the raw info struct from the name
            let (raw, name) = v.split_at(f_size);

            // If `raw` is empty, error
            if raw.len() < f_size {
                return Err(EfiStatus::BUFFER_TOO_SMALL.into());
            }

            // Initialize the new info struct
            info.as_mut_ptr()
                .cast::<u8>()
                .copy_from_nonoverlapping(raw.as_ptr() as *const u8, f_size);
            let info = info.assume_init();

            // The length of the filename in UTF-16, minus the nul terminator.
            let name_len = (name.len() / 2) - 1;

            // Rebind `name` from a `&[u8]` slice
            // to a `&[u16]` slice of half the length
            let name = from_raw_parts(name.as_ptr() as *const u16, name_len);

            // Then decode it as UTF-16
            let name = char::decode_utf16(name.iter().copied())
                .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect::<String>();
            Ok(FileInfo::new(info, name))
        }
    }

    pub fn directory(&self) -> bool {
        (self.info.flags & Self::DIRECTORY) == Self::DIRECTORY
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
