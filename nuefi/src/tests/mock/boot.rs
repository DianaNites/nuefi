extern crate std;

use alloc::boxed::Box;
use core::{
    ffi::c_void,
    mem::{size_of, MaybeUninit},
    ptr::{null_mut, NonNull},
};

use memoffset::offset_of;
use nuefi_core::{
    base::{Char16, Guid},
    error::Status,
    table::{BootServices, Header, CRC},
};

use super::System;
use crate::{
    get_boot_table,
    proto::console::raw::RawSimpleTextOutput,
    string::UcsString,
    tests::mock::to_bytes,
};

#[derive(Debug)]
#[repr(C)]
pub struct MockBoot {
    pub this: BootServices,
}

impl MockBoot {
    pub fn new() -> Self {
        const MOCK_HEADER: Header = Header {
            signature: BootServices::SIGNATURE,
            revision: BootServices::REVISION,
            size: size_of::<BootServices>() as u32,
            crc32: 0,
            reserved: 0,
        };
        let mut t: BootServices = unsafe { MaybeUninit::zeroed().assume_init() };
        t.header = MOCK_HEADER;
        t.locate_protocol = Some(Self::locate_protocol);

        t.header.crc32 = {
            let mut digest = CRC.digest();
            // Safety: We ensure in the definition that there is no uninit padding.
            unsafe { digest.update(to_bytes(&t)) };
            digest.finalize()
        };

        Self { this: t }
    }
}

impl MockBoot {
    unsafe extern "efiapi" fn locate_protocol(
        guid: *mut Guid,
        key: *mut c_void,
        out: *mut *mut c_void,
    ) -> Status {
        if out.is_null() || guid.is_null() {
            return Status::INVALID_PARAMETER;
        }
        let guid = *guid;
        let out = &mut *out;

        // It's okay to use this because it will only be called after
        // we're set up, by which point our main has set these up.
        if let Some(st) = get_boot_table() {
            let off = offset_of!(System, sys) as isize;
            // Get our parent System, which contains the SystemTable and also us.
            let sys = &*st.raw().cast::<u8>().offset(-off).cast::<System>();

            let found = sys
                .db
                .iter()
                .find_map(|h| h.protos.iter().find(|p| p.guid == guid));

            std::dbg!(&sys);
            std::dbg!(&found);

            if let Some(proto) = found {
                *out = proto.ptr.cast_mut().cast();
                Status::SUCCESS
            } else {
                Status::NOT_FOUND
            }
        } else {
            Status::UNSUPPORTED
        }
    }
}
