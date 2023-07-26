use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    any::Any,
    mem::size_of,
    ptr::{addr_of, addr_of_mut, null_mut},
};

use nuefi_core::{
    base::Char16,
    table::{Header, CRC},
};

use crate::{
    error::Status,
    proto::{
        self,
        console::raw::RawSimpleTextOutput,
        graphics::{raw::RawGraphicsOutput, GraphicsOutput},
        Protocol,
    },
    table::raw::{RawBootServices, RawRuntimeServices, RawSystemTable, Revision},
    EfiHandle,
};

const MOCK_REVISION: Revision = Revision::new(2, 7, 0);
const MOCK_FW_REVISION: u32 = 69420;
pub const MOCK_VENDOR: &str = "Mock Vendor";

const fn mock_boot() -> RawBootServices {
    const MOCK_HEADER: Header = Header {
        signature: RawBootServices::SIGNATURE,
        revision: MOCK_REVISION,
        size: size_of::<RawBootServices>() as u32,
        crc32: 0,
        reserved: 0,
    };
    let b = [0u8; size_of::<RawBootServices>()];
    // Safety:
    // - All fields of `RawBootServices` are safely nullable/zero
    //
    // use transmute because `zeroed` is not const.
    let mut t: RawBootServices = unsafe { core::mem::transmute::<_, _>(b) };
    t.header = MOCK_HEADER;
    t
}

const fn mock_run() -> RawRuntimeServices {
    const MOCK_HEADER: Header = Header {
        signature: RawRuntimeServices::SIGNATURE,
        revision: MOCK_REVISION,
        size: size_of::<RawRuntimeServices>() as u32,
        crc32: 0,
        reserved: 0,
    };
    let b = [0u8; size_of::<RawRuntimeServices>()];
    // Safety:
    // - All fields of `RawRuntimeServices` are safely nullable/zero
    let mut t: RawRuntimeServices = unsafe { core::mem::transmute::<_, _>(b) };
    t.header = MOCK_HEADER;
    t
}

const fn mock_out() -> RawSimpleTextOutput {
    unsafe extern "efiapi" fn reset(this: *mut RawSimpleTextOutput, extended: bool) -> Status {
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn output_string(
        this: *mut RawSimpleTextOutput,
        string: *const Char16,
    ) -> Status {
        Status::SUCCESS
    }

    unsafe extern "efiapi" fn clear_screen(this: *mut RawSimpleTextOutput) -> Status {
        Status::SUCCESS
    }

    RawSimpleTextOutput {
        reset: Some(reset),
        output_string: Some(output_string),
        test_string: None,
        query_mode: None,
        set_mode: None,
        set_attribute: None,
        clear_screen: Some(clear_screen),
        set_cursor_position: None,
        enable_cursor: None,
        mode: null_mut(),
    }
}

const fn mock_gop() -> RawGraphicsOutput {
    unsafe extern "efiapi" fn set_mode(this: *mut RawGraphicsOutput, mode: u32) -> Status {
        Status::DEVICE_ERROR
    }

    RawGraphicsOutput {
        query_mode: None,
        set_mode: Some(set_mode),
        blt: None,
        mode: null_mut(),
    }
}

const fn mock_system() -> RawSystemTable {
    const MOCK_HEADER: Header = Header {
        signature: RawSystemTable::SIGNATURE,
        revision: MOCK_REVISION,
        size: size_of::<RawSystemTable>() as u32,
        crc32: 0,
        reserved: 0,
    };
    RawSystemTable {
        header: MOCK_HEADER,
        firmware_vendor: null_mut(),
        firmware_revision: MOCK_FW_REVISION,
        console_in_handle: EfiHandle::null(),
        con_in: null_mut(),
        console_out_handle: EfiHandle::null(),
        con_out: null_mut(),
        console_err_handle: EfiHandle::null(),
        con_err: null_mut(),
        runtime_services: null_mut(),
        boot_services: null_mut(),
        number_of_table_entries: 0,
        configuration_table: null_mut(),
        _pad1: [0u8; 4],
    }
}

/// # Safety:
///
/// `T` must not have uninit padding.
const unsafe fn to_bytes<T>(this: &T) -> &[u8] {
    // Safety: `this` is valid by definition
    // Lifetime is bound to `this`
    unsafe { core::slice::from_raw_parts(this as *const T as *const u8, size_of::<T>()) }
}

/// Create mock implementations of a SystemTable and a few protocols
/// to aid testing of the basic interactions
pub fn mock() -> (Box<RawSystemTable>, Vec<Box<dyn Any>>) {
    let mut vendor = MOCK_VENDOR.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let mut system = Box::new(mock_system());
    let mut boot = Box::new(mock_boot());
    let mut run = Box::new(mock_run());
    let mut out = Box::new(mock_out());

    boot.locate_protocol = Some(locate_protocol);

    boot.header.crc32 = {
        let mut digest = CRC.digest();
        // Safety: We ensure in the definition that there is no uninit padding.
        unsafe { digest.update(to_bytes(&*boot)) };
        digest.finalize()
    };

    run.header.crc32 = {
        let mut digest = CRC.digest();
        // Safety: We ensure in the definition that there is no uninit padding.
        unsafe { digest.update(to_bytes(&*run)) };
        digest.finalize()
    };

    system.boot_services = addr_of_mut!(*boot).cast();
    system.runtime_services = addr_of_mut!(*run).cast();
    system.con_out = addr_of_mut!(*out).cast();
    // system.firmware_vendor = addr_of!(vendor[0]);
    system.firmware_vendor = vendor.as_ptr().cast_mut();

    system.header.crc32 = {
        let mut digest = CRC.digest();
        // Safety: We ensure in the definition that there is no uninit padding.
        unsafe { digest.update(to_bytes(&*system)) };
        digest.finalize()
    };

    (
        system,
        vec![
            //
            boot,
            out,
            run,
            Box::new(vendor),
        ],
    )
}

use imps::*;
mod imps {
    use core::ffi::c_void;

    use super::*;

    pub static mut MOCK_GOP: RawGraphicsOutput = mock_gop();

    pub unsafe extern "efiapi" fn locate_protocol(
        guid: *mut proto::Guid,
        key: *mut c_void,
        out: *mut *mut c_void,
    ) -> Status {
        let guid = *guid;
        if guid == GraphicsOutput::GUID {
            out.write(addr_of_mut!(MOCK_GOP) as *mut _);
            Status::SUCCESS
        } else {
            out.write(null_mut());
            Status::NOT_FOUND
        }
    }
}
