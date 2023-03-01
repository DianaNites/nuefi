//! UEFI Device Path Protocol

use super::{Guid, Protocol};
use crate::{
    error::{EfiStatus, Result, UefiError},
    get_boot_table,
    string::UefiString,
    table::BootServices,
    util::interface,
    Protocol,
};

pub mod raw;
use alloc::string::String;

use raw::{RawDevicePath, RawDevicePathToText, RawDevicePathUtil};

interface!(
    #[Protocol("09576E91-6D3F-11D2-8E39-00A0C969723B", crate("crate"))]
    DevicePath(RawDevicePath)
);

impl<'table> DevicePath<'table> {
    /// Free the DevicePath
    pub(crate) fn free(&mut self, boot: &BootServices) -> Result<()> {
        // Safety: Construction ensures these are valid
        unsafe { boot.free_pool(self.interface as *mut u8) }
    }

    /// Get this DevicePath as a String, if possible.
    pub fn to_string(&self) -> Result<String> {
        if let Some(table) = get_boot_table() {
            let boot = table.boot();
            // TODO: Implement DevicePath ourselves in pure Rust and just do it ourselves?
            let util = boot.locate_protocol::<DevicePathToText>()?.unwrap();
            let s = util.convert_device_path_to_text(self)?;
            let s = s.to_string();
            Ok(s)
        } else {
            Err(EfiStatus::DEVICE_ERROR.into())
        }
    }
}

interface!(
    #[Protocol("0379BE4E-D706-437D-B037-EDB82FB772A4", crate("crate"))]
    DevicePathUtil(RawDevicePathUtil)
);

impl<'table> DevicePathUtil<'table> {
    /// [DevicePath] size, in bytes. NOT including the End Of Path node.
    pub fn get_device_path_size(&self, node: &DevicePath) -> usize {
        // Safety: Construction ensures these are valid
        unsafe {
            (self.interface().get_device_path_size.unwrap())(node.interface)
                // End of path node
                - core::mem::size_of::<RawDevicePath>()
        }
    }

    /// Duplicate/Clone the [DevicePath] `path`
    pub fn duplicate(&self, path: &DevicePath) -> Result<DevicePath> {
        // Safety: Construction ensures these are valid
        let ret = unsafe {
            //
            (self.interface().duplicate_device_path.unwrap())(path.interface)
        };
        if !ret.is_null() {
            // Safety: ret is non-null
            unsafe { Ok(DevicePath::from_raw(ret)) }
        } else {
            Err(EfiStatus::OUT_OF_RESOURCES.into())
        }
    }
}

interface!(
    #[Protocol("8B843E20-8132-4852-90CC-551A4E4A7F1C", crate("crate"))]
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
    pub fn convert_device_node_to_text(&self, node: &DevicePath) -> Result<UefiString> {
        // Safety: construction ensures correctness
        let ret = unsafe {
            //
            (self.interface().convert_device_node_to_text.unwrap())(node.interface, false, false)
        };
        if !ret.is_null() {
            // Safety: `ret` is a non-null owned UEFI string
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(UefiError::new(EfiStatus::OUT_OF_RESOURCES))
        }
    }

    /// Returns an owned [UefiString] of `path`
    ///
    /// # Errors
    ///
    /// - If memory allocation fails
    pub fn convert_device_path_to_text(&self, path: &DevicePath) -> Result<UefiString> {
        // Safety: construction ensures correctness
        let ret = unsafe {
            //
            (self.interface().convert_device_path_to_text.unwrap())(path.interface, false, false)
        };
        if !ret.is_null() {
            // Safety: `ret` is a non-null owned UEFI string
            Ok(unsafe { UefiString::from_ptr(ret) })
        } else {
            Err(UefiError::new(EfiStatus::OUT_OF_RESOURCES))
        }
    }
}
