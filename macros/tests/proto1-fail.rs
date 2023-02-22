//! Test that an obviously invalid GUID fails
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]
use core::ptr::null_mut;

use ::uefi::Protocol;
use nuuid::Uuid;

// Random UUID from `uuidgen`
const GUID: &str = "c986ec27-69420-af54-4b55-80aa-91697fcdf8eb";

#[repr(C)]
struct RawProto {
    pro: *mut RawProto,
}

#[Protocol("c986ec27-69420-af54-4b55-80aa-91697fcdf8eb")]
#[repr(C)]
struct Proto(RawProto);

impl Proto {
    fn new(pro: *mut RawProto) -> Self {
        Self(RawProto { pro })
    }
}

fn main() {
    let p = Proto::new(null_mut());
}
