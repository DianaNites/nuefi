extern crate std;
use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    alloc::Layout,
    any::Any,
    fmt::Debug,
    mem::size_of,
    ptr::{addr_of, addr_of_mut, null_mut},
};

use nuefi_core::{
    base::{Char16, Guid},
    table::{Header, CRC},
};

use crate::{
    error::Status,
    proto::{
        self,
        console::{raw::RawSimpleTextOutput, SimpleTextOutput},
        graphics::{raw::RawGraphicsOutput, GraphicsOutput},
        Protocol,
    },
    string::UcsString,
    table::{
        raw::{RawBootServices, RawRuntimeServices, RawSystemTable, Revision},
        BootServices,
    },
    EfiHandle,
};

mod boot;
mod console;

const MOCK_REVISION: Revision = Revision::new(2, 7, 0);
const MOCK_FW_REVISION: u32 = 69420;
pub const MOCK_VENDOR: &str = "Mock Vendor";

/// Points to an instance of a Protocol, type-erased
#[derive(Debug)]
pub struct ProtoEntry {
    /// GUID of this Protocol
    pub guid: Guid,

    /// Pointer to this Protocol instance
    pub ptr: *const u8,

    pub free: unsafe fn(*const u8),

    /// Layout for the structure behind `ptr`
    pub layout: Layout,
}

/// Points to an instance of a Protocol, type-erased
///
/// Pointers to this are what we use as [`EfiHandle`],
/// thus their address must be stable.
#[derive(Debug)]
pub struct HandleEntry {
    pub protos: Vec<ProtoEntry>,
}

#[derive(Debug)]
pub struct System {
    #[allow(clippy::vec_box)]
    pub db: Vec<Box<HandleEntry>>,

    pub sys: RawSystemTable,

    pub boot: Box<MockBoot>,

    pub run: Box<RawRuntimeServices>,

    pub vendor: UcsString,
}

impl System {
    fn new() -> Box<Self> {
        let vendor = UcsString::new(MOCK_VENDOR);
        let mut boot = Box::new(MockBoot::new());

        let mut run = Box::new(mock_run());
        run.header.crc32 = {
            let mut digest = CRC.digest();
            // Safety: We ensure in the definition that there is no uninit padding.
            unsafe { digest.update(to_bytes(&*run)) };
            digest.finalize()
        };

        let out = Box::into_raw(Box::new(MockConsole::new()));
        let mut console = Box::new(HandleEntry { protos: Vec::new() });
        let console_out_handle = &mut *console as *mut HandleEntry;

        console.protos.push(ProtoEntry {
            guid: SimpleTextOutput::GUID,
            ptr: out.cast(),
            free: MockConsole::free,
            layout: Layout::new::<MockConsole>(),
        });

        // Safety: We are UEFI
        let mut sys = unsafe {
            RawSystemTable {
                header: Header {
                    signature: RawSystemTable::SIGNATURE,
                    revision: MOCK_REVISION,
                    size: size_of::<RawSystemTable>() as u32,
                    crc32: 0,
                    reserved: 0,
                },
                // Note: This is fine because `vendor` is part of `system`,
                // and we know `vendor` is allocated on our heap.
                // It's pointer will not be invalidated while the SystemTable lives.
                firmware_vendor: vendor.as_ptr().cast_mut(),
                firmware_revision: MOCK_FW_REVISION,
                console_in_handle: EfiHandle::null(),
                con_in: null_mut(),

                // console_out_handle: EfiHandle::null(),
                console_out_handle: EfiHandle::new(console_out_handle.cast()),
                con_out: out.cast(),

                console_err_handle: EfiHandle::null(),
                con_err: null_mut(),

                // Both of these are safe because we know their pointers are stable
                runtime_services: &mut *run as *mut RawRuntimeServices,
                boot_services: &mut boot.this,

                number_of_table_entries: 0,
                configuration_table: null_mut(),
                _pad1: [0u8; 4],
            }
        };

        let mut sys = Box::new(Self {
            db: vec![
                // .
                console,
            ],
            vendor,
            boot,
            run,
            sys,
        });

        sys
    }

    fn add_protocol(&mut self, handle: EfiHandle, entry: ProtoEntry) {
        //
    }
}

impl Drop for System {
    fn drop(&mut self) {
        // Drop all protocol instances
        for handle in &self.db {
            for proto in &handle.protos {
                let ptr = proto.ptr;
                let layout = proto.layout;

                // Safety:
                // - Internally ensured to always be valid for this operation
                unsafe {
                    (proto.free)(ptr);
                    unsafe { alloc::alloc::dealloc(ptr.cast_mut(), layout) }
                }
            }
        }
    }
}

const fn mock_run() -> RawRuntimeServices {
    const MOCK_HEADER: Header = Header {
        signature: RawRuntimeServices::SIGNATURE,
        revision: RawRuntimeServices::REVISION,
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
// pub fn mock() -> (Box<RawSystemTable>, Vec<Box<dyn Any>>) {
pub fn mock() -> Box<System> {
    let mut sys = System::new();
    let vendor = &mut sys.vendor;
    let system = &mut sys.sys;
    let boot = &mut sys.boot;
    let run = &mut sys.run;

    system.header.crc32 = {
        let mut digest = CRC.digest();
        // Safety: We ensure in the definition that there is no uninit padding.
        unsafe { digest.update(to_bytes(&*system)) };
        digest.finalize()
    };

    #[allow(clippy::needless_return)]
    return sys;

    // boot.locate_protocol = Some(locate_protocol);

    // boot.header.crc32 = {
    //     let mut digest = CRC.digest();
    //     // Safety: We ensure in the definition that there is no uninit padding.
    //     unsafe { digest.update(to_bytes(&*boot)) };
    //     digest.finalize()
    // };

    // run.header.crc32 = {
    //     let mut digest = CRC.digest();
    //     // Safety: We ensure in the definition that there is no uninit padding.
    //     unsafe { digest.update(to_bytes(&*run)) };
    //     digest.finalize()
    // };

    // system.boot_services = addr_of_mut!(*boot).cast();
    // system.runtime_services = addr_of_mut!(*run).cast();
    // system.con_out = addr_of_mut!(*out).cast();
    // // system.firmware_vendor = addr_of!(vendor[0]);
    // system.firmware_vendor = vendor.as_ptr().cast_mut();

    // system.header.crc32 = {
    //     let mut digest = CRC.digest();
    //     // Safety: We ensure in the definition that there is no uninit padding.
    //     unsafe { digest.update(to_bytes(&*system)) };
    //     digest.finalize()
    // };

    #[cfg(no)]
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

use self::{boot::MockBoot, console::MockConsole};
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
