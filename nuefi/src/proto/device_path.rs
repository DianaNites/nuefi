//! UEFI Device Path Protocol
//!
//! # References
//!
//! - [UEFI Section 10. Device Path Protocol][s10]
//!
//! [s10]: <https://uefi.org/specs/UEFI/2.10/10_Protocols_Device_Path_Protocol.html>

use alloc::string::{String, ToString};
use core::{ffi::c_void, mem::transmute, ptr::addr_of};

use nuefi_core::proto::device_path::{nodes::End, DevicePathHdr};

pub mod raw {
    // FIXME: Ugly hack to keep things compiling
    pub use nuefi_core::proto::device_path::{
        DevicePathHdr as RawDevicePath,
        DevicePathToText as RawDevicePathToText,
        DevicePathUtil as RawDevicePathUtil,
    };
}
use raw::{RawDevicePathToText, RawDevicePathUtil};

use super::{Protocol, Scope};
use crate::{
    error::{Result, Status},
    get_boot_table,
    mem::MemoryType,
    nuefi_core::interface,
    string::UefiString,
    table::BootServices,
    Protocol,
};

/// Helper to get [`DevicePathUtil`]
fn _get_dev_util<'proto>(
    _t: &'proto DevicePath<'_>,
) -> Result<Scope<'proto, DevicePathUtil<'proto>>> {
    if let Some(table) = get_boot_table() {
        let boot = table.boot();
        let util = boot.get_protocol::<DevicePathUtil>()?;

        // Safety: This is required because our local table is an implementation
        // detail.
        //
        // The correct lifetime is `'proto`,
        // referencing the DevicePath calling us.
        unsafe { Ok(transmute(util)) }
    } else {
        Err(Status::UNSUPPORTED.into())
    }
}

/// Helper to get [`DevicePathToText`]
fn get_dev_text<'proto>(
    _t: &'proto DevicePath<'_>,
) -> Result<Scope<'proto, DevicePathToText<'proto>>> {
    if let Some(table) = get_boot_table() {
        let boot = table.boot();
        let util = boot.get_protocol::<DevicePathToText>()?;

        // Safety: This is required because our local table is an implementation
        // detail.
        //
        // The correct lifetime is `'proto`,
        // referencing the DevicePath calling us.
        unsafe { Ok(transmute(util)) }
    } else {
        Err(Status::UNSUPPORTED.into())
    }
}

interface!(
    #[Protocol("09576E91-6D3F-11D2-8E39-00A0C969723B")]
    DevicePath(DevicePathHdr)
);

impl<'table> DevicePath<'table> {
    /// Free the DevicePath
    ///
    /// # Safety
    ///
    /// - `self` must *own* the DevicePath
    /// - No other references to this path may exist
    /// - This must only be called once.
    ///
    /// This is intended to be used from [`Drop`], and that is why it takes
    /// `&mut Self`.
    pub unsafe fn free(&mut self, boot: &BootServices) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { boot.free_pool(self.interface as *mut c_void) }
    }

    /// Total size, in bytes, of the entire DevicePath structure, including all
    /// instances.
    ///
    /// This will go through the entire structure to determine the size.
    /// Repeated calls should be avoided.
    pub fn len(&self) -> usize {
        let mut size = 0;
        let mut ptr = self.interface as *const DevicePathHdr;
        // Safety:
        // - Existence of `self` implies this is a valid `DevicePath`
        // - `DevicePath`s must end with an `End` node.
        // - `DevicePathHdr` has no alignment requirements
        unsafe {
            while End::entire() != unsafe { *ptr } {
                let len: usize = u16::from_le_bytes((*ptr).len).into();
                ptr = (ptr as *const u8).add(len) as *const _;
                size += len;
            }
        }
        size + size_of::<End>()
    }

    /// Duplicate/clone the path
    ///
    /// It is the callers responsibility to free the returned [`DevicePath`]
    ///
    /// This does **not** use [`DevicePathUtil::duplicate`]
    pub fn duplicate(&self) -> Result<DevicePath<'table>> {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();

            let len = self.len();
            let mem = boot.allocate_pool(MemoryType::LOADER_DATA, len)?;

            // Safety: Both pointers are valid for the same length
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.interface as *const u8,
                    mem.as_ptr().cast(),
                    len,
                )
            };

            // Safety:
            // - `mem` is now a copy of `self`
            // - This is required because our local table is an implementation detail
            // The correct lifetime is `'table`
            unsafe { Ok(transmute(mem)) }
        } else {
            Err(Status::DEVICE_ERROR.into())
        }
    }

    /// Get this DevicePath as a [`UefiString`] using [`DevicePathToText`]
    pub fn to_uefi_string(&self) -> Result<UefiString> {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();
            // TODO: Implement DevicePath ourselves in pure Rust and just do it ourselves?
            let util = get_dev_text(self)?;
            let s = util.convert_device_path_to_text(self)?;
            Ok(s)
        } else {
            Err(Status::DEVICE_ERROR.into())
        }
    }

    /// Get this DevicePath as a [`String`] using [`DevicePathToText`]
    pub fn to_string(&self) -> Result<String> {
        Ok(self.to_uefi_string()?.to_string())
    }

    /// Append `node` to ourselves, returning a new path.
    // FIXME: These leak memory.
    #[cfg(no)]
    pub fn append(&self, node: &DevicePath) -> Result<DevicePath<'table>> {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();
            // TODO: Implement DevicePath ourselves in pure Rust and just do it ourselves?
            let util = get_dev_util(self)?;
            let s = util.append(self, node);
            // Safety: This is required because our local table is an implementation detail
            // The correct lifetime is `'table`
            unsafe { Ok(transmute(s)) }
        } else {
            Err(Status::DEVICE_ERROR.into())
        }
    }

    /// Append the UEFI file path, returning the new device path
    // FIXME: These leak memory.
    #[cfg(no)]
    pub fn append_file_path(&self, path: &str) -> Result<DevicePath<'table>> {
        let table = get_boot_table().ok_or(Status::UNSUPPORTED)?;
        let boot = table.boot();
        // log::trace!("Path: {path}");

        let hdr_size = size_of::<RawDevicePath>();
        let path: Vec<u16> = path.encode_utf16().chain([0]).collect();
        let path_len = path.len() * 2;

        let cap = path_len + hdr_size + hdr_size;
        // log::trace!("Capacity: {cap} - {path_len}");

        let data = boot
            .allocate_pool(MemoryType::LOADER_DATA, cap)?
            .cast::<u8>();

        let path_len = path_len.try_into().map_err(|_| Status::BAD_BUFFER_SIZE)?;

        let media = File::new_header(path_len);
        let end = End::entire();

        // Safety: `data` is valid for `cap`, which is all we write
        unsafe {
            // Write Media file node
            let ptr = &media as *const _ as *const u8;
            data.as_ptr().copy_from_nonoverlapping(ptr, hdr_size);

            // Write name
            let ptr = path.as_ptr() as *const u8;
            let name = data.as_ptr().add(hdr_size);
            name.copy_from_nonoverlapping(ptr, path_len.into());

            // Write end of structure node
            let ptr = &end as *const _ as *const u8;
            let eos = data.as_ptr().add(hdr_size + path_len as usize);
            eos.copy_from_nonoverlapping(ptr, hdr_size);

            // We've ensured this is a valid `DevicePath` structure
            let node = unsafe { DevicePath::new(data.as_ptr() as *mut _) };
            // log::trace!("Node: {:#?}", node.to_string());

            // Append it
            let ret = self.append(&node)?;

            // Free our data
            boot.free_pool(data.as_ptr().cast())?;

            Ok(ret)
        }
    }
}

interface!(
    #[Protocol("0379BE4E-D706-437D-B037-EDB82FB772A4")]
    DevicePathUtil(RawDevicePathUtil)
);

impl<'table> DevicePathUtil<'table> {
    /// [DevicePath] size, in bytes. NOT including the End Of Path node.
    pub fn get_device_path_size(&self, node: &DevicePath) -> usize {
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().get_device_path_size.unwrap())(node.interface)
                // End of path node
                - core::mem::size_of::<DevicePathHdr>()
        }
    }

    /// Duplicate/Clone the [DevicePath] `path`
    pub fn duplicate(&self, path: &DevicePath) -> Result<DevicePath<'table>> {
        // Safety: Construction ensures these are valid
        let ret = unsafe {
            //
            (self.interface().duplicate_device_path.unwrap())(path.interface)
        };
        if !ret.is_null() {
            // Safety: ret is non-null
            unsafe { Ok(DevicePath::from_raw(ret)) }
        } else {
            Err(Status::OUT_OF_RESOURCES.into())
        }
    }

    /// Append the specified [`DevicePath`] *node*
    pub fn append(&self, path: &DevicePath, node: &DevicePath) -> DevicePath<'table> {
        // Safety: Construction ensures these are valid
        let ret = unsafe {
            (self.interface().append_device_node.unwrap())(path.interface, node.interface)
        };
        assert!(!ret.is_null(), "appended device path was null");
        // Safety: ret is non-null
        unsafe { DevicePath::from_raw(ret) }
    }
}

interface!(
    #[Protocol("8B843E20-8132-4852-90CC-551A4E4A7F1C")]
    DevicePathToText(RawDevicePathToText)
);

impl<'table> DevicePathToText<'table> {
    /// Returns an owned [UefiString] of `node`, a component of a [DevicePath]
    ///
    /// With the path `PciRoot(0x0)/Pci(0x1F,0x2)/Sata(0x0,0xFFFF,0x0)`,
    /// this would return `PciRoot(0x0)`.
    ///
    /// # Errors
    ///
    /// - If memory allocation fails
    pub fn convert_device_node_to_text(&self, node: &DevicePath) -> Result<UefiString<'table>> {
        // Safety: construction ensures correctness
        let ret = unsafe {
            //
            (self.interface().convert_device_node_to_text.unwrap())(node.interface, false, false)
        };
        if !ret.is_null() {
            // Safety: `ret` is a non-null owned UEFI string
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(Status::OUT_OF_RESOURCES.into())
        }
    }

    /// Returns an owned [UefiString] of `path`
    ///
    /// # Errors
    ///
    /// - If memory allocation fails
    pub fn convert_device_path_to_text(&self, path: &DevicePath) -> Result<UefiString<'table>> {
        // Safety: construction ensures correctness
        let ret = unsafe {
            //
            (self.interface().convert_device_path_to_text.unwrap())(path.interface, false, false)
        };
        if !ret.is_null() {
            // Safety: `ret` is a non-null owned UEFI string
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(Status::OUT_OF_RESOURCES.into())
        }
    }
}

mod seal {
    use super::DevicePath;

    pub trait Sealed {}

    impl<'table> Sealed for DevicePath<'table> {}
    impl<'table, 'a> Sealed for &'a DevicePath<'table> {}
}

/// Represents something that can be represented as a [`DevicePath`]
pub trait AsDevicePath<'table>: seal::Sealed {
    //
    fn as_device_path(&self) -> &DevicePath<'table>;
}
