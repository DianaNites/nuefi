//! UEFI Media protocols
use alloc::{string::String, vec::Vec};
use core::{
    cell::Cell,
    iter::{from_fn, once},
    marker::PhantomData,
    mem::{size_of, MaybeUninit},
    ptr::null_mut,
    slice::from_raw_parts,
};

use log::trace;
use macros::GUID;
use raw::*;

use crate::{
    error::{EfiStatus, Result, UefiError},
    proto::{Entity, Guid, Protocol},
    util::interface,
    Protocol,
};

pub mod raw;

interface!(
    #[Protocol("4006C0C1-FCB3-403E-996D-4A6C8724E06D", crate("crate"))]
    LoadFile2(RawLoadFile2)
);

impl<'table> LoadFile2<'table> {
    //
}

interface!(
    /// UEFI Simple filesystem protocol.
    /// Gives [`FsHandle`] based access to a device.
    ///
    /// UEFI supports the `FAT{12,16,32}` filesystems out of the box
    #[Protocol("964E5B22-6459-11D2-8E39-00A0C969723B", crate("crate"))]
    SimpleFileSystem(RawSimpleFileSystem)
);

impl<'table> SimpleFileSystem<'table> {
    /// Open the root directory of a volume
    pub fn open_volume<'h>(&self) -> Result<FsHandle<'h, 'table>> {
        let mut out = null_mut();
        // Safety: `file` is always valid, checked for null
        // anything else is the responsibility of firmware
        let ret = unsafe { (self.interface().open_volume.unwrap())(self.interface, &mut out) };
        if ret.is_success() {
            assert!(
                !out.is_null(),
                "SimpleFileSystem returned success, but the file was null.",
            );
            // Safety: `FsHandle` isn't a Protocol
            unsafe { Ok(FsHandle::new(out)) }
        } else {
            Err(UefiError::new(ret))
        }
    }
}

// Terrible hacks
mod file_imp {
    use super::*;

    interface!(FileImp(RawFsHandle));
}
use file_imp::FileImp;

/// UEFI File Protocol
///
/// This represents an entity on the filesystem, files and directories.
///
/// This will call [`FsHandle::close`] on [`Drop`]
///
/// See [`SimpleFileSystem`]
// The `'this` lifetime is independent and under `'table`
// because the `FsHandle` is independent of whatever created it,
// only depending on the BootServices/SystemTable
//
// For example, just because `SimpleFileSystem` went out of scope does not
// invalidate this handle.
pub struct FsHandle<'this, 'table>
where
    'table: 'this,
{
    raw: FileImp<'this>,
    interface: *mut RawFsHandle,
    closed: Cell<bool>,

    /// Holds the BootServices table lifetime
    table: PhantomData<&'table mut ()>,
}

// interface hacks
impl<'this, 'table> FsHandle<'this, 'table> {
    pub(crate) unsafe fn new(interface: *mut RawFsHandle) -> Self {
        Self {
            raw: FileImp::new(interface),
            interface,
            closed: Cell::new(false),
            table: PhantomData,
        }
    }

    fn interface(&self) -> &RawFsHandle {
        // SAFETY:
        // Ensured valid in construction.
        // Continued validity ensured by the type system
        // Should be statically impossible to invalidate
        unsafe { &*(self.interface.cast_const()) }
    }
}

// Internal
impl<'this, 'table> FsHandle<'this, 'table> {
    // Use a new lifetime because this is a new handle independent of ours.
    fn open_impl<'new_this>(
        &self,
        name: &str,
        mode: u64,
        flags: u64,
    ) -> Result<FsHandle<'new_this, 'table>> {
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
            // Safety: `FsHandle` isn't a Protocol
            unsafe { Ok(FsHandle::new(out)) }
        } else {
            Err(UefiError::new(ret))
        }
    }

    /// Reads the buffer for [`FsHandle::read_impl`]
    fn read_impl_size(&self) -> Result<usize> {
        let rd = self.interface().read.unwrap();
        let mut size = 0;

        // Calling to get buffer size

        // `interface` and `size` are always valid
        // ptr is null and thats okay
        let ret = unsafe { (rd)(self.interface, &mut size, null_mut()) };

        if size == 0 && ret.is_success() {
            // End of Directories/File
            Ok(size)
        } else if ret == EfiStatus::BUFFER_TOO_SMALL {
            let _ = return Ok(size);
        } else {
            // Anything other than `BUFFER_TOO_SMALL` here is an error
            Err(UefiError::new(ret))
        }
    }

    /// Reads the buffer for [`FsHandle::read_impl`].
    /// Returns how many bytes were written.
    ///
    /// # Safety
    ///
    /// - `out` must be valid for `size` bytes
    unsafe fn read_impl_write(&self, size: usize, out: &mut [u8]) -> Result<usize> {
        let mut size = size;
        let rd = self.interface().read.unwrap();
        let ptr = out.as_mut_ptr();

        // `interface`, `size`, are valid
        // `ptr` is valid for `size` bytes
        let ret = (rd)(self.interface, &mut size, ptr);

        if ret.is_success() {
            Ok(size)
        } else {
            Err(ret.into())
        }
    }

    /// Implementation of the `read` call. Returns how many bytes written
    ///
    /// Takes `out` as input, expects it to be an empty vector, and will be
    /// resized.
    fn read_impl(&self, out: &mut Vec<u8>) -> Result<usize> {
        assert!(out.is_empty(), "Expected `out` to be empty");
        // Safety: Described within
        unsafe {
            let rd = self.interface().read.unwrap();

            // Calling to get buffer size
            let mut size = self.read_impl_size()?;

            // Here we reserve enough memory for `size`, initializing to `0`.
            out.resize(size, 0);

            // Assert just in case?
            assert!(out.capacity() >= size, "File read capacity bug");

            // Calling to fill the buffer
            match self.read_impl_write(size, out) {
                Ok(n) => Ok(size),
                Err(e) => Err(e),
            }
        }
    }
}

// Deeper abstractions around UEFI
impl<'this, 'table> FsHandle<'this, 'table> {
    /// Return `Ok(true)` if entity exists
    pub fn try_exists(&self) -> Result<bool> {
        let ret = self.info();
        match ret {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.status() == EfiStatus::NOT_FOUND {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Return `true` if entity exists, `false` otherwise
    pub fn exists(&self) -> bool {
        self.try_exists().unwrap_or_default()
    }

    /// Read to `buf` until the end of the file,
    /// returning how many bytes were read.
    pub fn read_to_end(&self, buf: &mut Vec<u8>) -> Result<usize> {
        let info = self.info()?;
        if info.directory() {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }
        let size: usize = info
            .size()
            .try_into()
            .map_err(|_| EfiStatus::DEVICE_ERROR)?;

        // Init the buffer for the size of the file
        buf.resize(size, 0);

        self.read(buf)
    }
}

// Relatively direct UEFI wrappers
impl<'this, 'table> FsHandle<'this, 'table> {
    /// Open a new [`FsHandle`] relative to this one
    ///
    /// Remember that UEFI paths use `\`, not `/`
    // FIXME: Provide a nice UEFI path type
    pub fn open<'new_this>(&self, name: &str) -> Result<FsHandle<'new_this, 'table>> {
        let mode = 0x1;
        let flags = 0;
        self.open_impl(name, mode, flags)
    }

    /// Create a new [`FsHandle`] relative to this one
    pub fn create<'new_this>(&self, name: &str) -> Result<FsHandle<'new_this, 'table>> {
        todo!()
    }

    /// Read the contents of the directory referred to by our handle
    ///
    /// This skips the `.` and `..` entries
    pub fn read_dir(&self) -> Result<impl Iterator<Item = Result<FsInfo>> + '_> {
        let info = self.info()?;
        if !info.directory() {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }

        let mut stop = false;

        let mut out: Vec<u8> = Vec::new();

        Ok(from_fn(move || loop {
            if stop {
                return None;
            }
            out.clear();

            let n = match self.read_impl(&mut out) {
                Ok(s) => s,
                Err(e) => return Some(Err(e)),
            };
            // Signals EOF
            if n == 0 {
                stop = true;
                if let Err(e) = self.set_position(0) {
                    return Some(Err(e));
                };
                return None;
            }

            let info = match FsInfo::from_bytes(out.clone()) {
                Ok(i) => i,
                Err(e) => return Some(Err(e)),
            };
            let name = info.name();
            if name == "." || name == ".." {
                continue;
            }
            break Some(Ok(info));
        }))
    }

    /// Read bytes into `buf`, returning how many were actually read.
    ///
    /// Less than requested may be read due to device error or EOF.
    ///
    /// The files current [`FsHandle::position`] increases by the amount read.
    pub fn read(&self, out: &mut [u8]) -> Result<usize> {
        let info = self.info()?;
        if info.directory() {
            return Err(EfiStatus::INVALID_PARAMETER.into());
        }
        let size = out.len();
        unsafe { self.read_impl_write(size, out) }
    }

    /// Information about this [`FsHandle`]. See [`FsInfo`]
    pub fn info(&self) -> Result<FsInfo> {
        let guid = FsInfo::GUID;
        let mut size: usize = 0;
        let mut out: Vec<u8> = Vec::new();

        // Safety: Described within
        unsafe {
            let fp = self.interface().get_info.unwrap();

            // Get the buffer size

            // All arguments are guaranteed valid
            let info = (fp)(self.interface, &guid, &mut size, null_mut());

            // It should be `BUFFER_TOO_SMALL`
            if info != EfiStatus::BUFFER_TOO_SMALL {
                return Err(UefiError::new(info));
            }
            // Sanity check
            if size == 0 {
                return Err(UefiError::new(EfiStatus::INVALID_PARAMETER));
            }

            // Reserve enough memory for `size`, initializing to `0`.
            out.resize(size, 0);

            // Just in case?
            assert!(out.capacity() >= size, "File::info capacity bug");

            let ptr = out.as_mut_ptr();

            // This time fill buffer

            // All arguments are guaranteed valid
            // `ptr` is valid for `size` bytes
            let info = (fp)(self.interface, &guid, &mut size, ptr);

            if info.is_success() {
                // We only call this on success, and before returning.
                // Out has been fully initialized, because we started initialized
                out.set_len(size);

                let info = FsInfo::from_bytes(out).unwrap();
                Ok(info)
            } else {
                Err(UefiError::new(info))
            }
        }
    }

    /// Close the handle, flushing all data, waiting for any pending async I/O.
    ///
    /// Does nothing if called multiple times
    pub fn close(&self) -> Result<()> {
        if self.closed.get() {
            return Ok(());
        }
        self.closed.set(true);
        // Safety: checked for null, anything else is the responsibility of firmware
        // This can only be called once.
        // Idk about real hardware yet, but
        // QEMU GP faults if this is called multiple times.
        // FIXME: QEMU/UEFI faults if `close` is called multiple times?
        unsafe { (self.interface().close.unwrap())(self.interface) }.into()
    }

    /// Flush all data with this handle
    pub fn flush(&self) -> Result<()> {
        // Safety: checked for null, anything else is the responsibility of firmware
        unsafe { (self.interface().flush.unwrap())(self.interface) }.into()
    }

    /// Set file cursor position
    pub fn set_position(&self, pos: u64) -> Result<()> {
        // Safety: statically valid
        unsafe { (self.interface().set_pos.unwrap())(self.interface, pos).into() }
    }

    /// Current file cursor position
    pub fn position(&self) -> Result<u64> {
        let mut pos: u64 = 0;
        // Safety: statically valid
        let ret = unsafe { (self.interface().get_pos.unwrap())(self.interface, &mut pos) };

        if ret.is_success() {
            Ok(pos)
        } else {
            Err(ret.into())
        }
    }
}

impl<'this, 'table> Drop for FsHandle<'this, 'table> {
    fn drop(&mut self) {
        if !self.closed.get() {
            self.closed.set(true);
            let _ = self.close();
        }
    }
}

/// UEFI [`FsHandle`] information
///
/// Represents information about an entity on the filesystem
#[GUID("09576E92-6D3F-11D2-8E39-00A0C969723B", crate("crate"))]
#[derive(Debug)]
pub struct FsInfo {
    info: RawFsInfo,
    name: String,
}

impl FsInfo {
    const DIRECTORY: u64 = 0x10;

    fn new(info: RawFsInfo, name: String) -> Self {
        Self { info, name }
    }

    /// Create `FsInfo` from bytes
    fn from_bytes(v: Vec<u8>) -> Result<FsInfo> {
        // Safety: Described within
        unsafe {
            let mut info: MaybeUninit<RawFsInfo> = MaybeUninit::uninit();
            let f_size = size_of::<RawFsInfo>();

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
            Ok(FsInfo::new(info, name))
        }
    }

    /// Is this a directory or not?
    pub fn directory(&self) -> bool {
        (self.info.flags & Self::DIRECTORY) == Self::DIRECTORY
    }

    /// Entity name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Entity filesystem size in bytes
    pub fn size(&self) -> u64 {
        self.info.file_size
    }

    /// Entity device size in bytes
    pub fn dev_size(&self) -> u64 {
        self.info.physical_size
    }
}
