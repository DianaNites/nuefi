//! Test that the Protocol macro basic features works correctly
use core::ptr::null_mut;

use nuefi::{proto::Protocol, Protocol};
use nuuid::Uuid;

// Random UUID from `uuidgen`
const GUID: &str = "c986ec27-af54-4b55-80aa-91697fcdf8eb";

#[repr(C)]
struct RawProto {
    pro: *mut RawProto,
}

#[Protocol("c986ec27-af54-4b55-80aa-91697fcdf8eb")]
#[derive(Debug)]
#[repr(transparent)]
struct Proto<'table> {
    /// .
    interface: *mut RawProto,
    phantom: core::marker::PhantomData<&'table mut RawProto>,
}

impl<'t> Proto<'t> {
    pub(crate) unsafe fn new(interface: *mut RawProto) -> Self {
        Self {
            interface,
            phantom: core::marker::PhantomData,
        }
    }
}

fn main() {
    let p = unsafe { Proto::new(null_mut()) };

    let mut buf = [0u8; 36];
    let s = Uuid::from_bytes_le(p.guid().to_bytes()).to_str(&mut buf);
    assert_eq!(s, GUID, "Protocol macro didn't do GUID correctly");

    let name = "Proto";

    // assert_eq!(Proto::NAME, name, "Protocol macro didn't do NAME correctly");
    assert_eq!(Proto::NAME, name, "Protocol macro didn't do NAME correctly");

    // println!("{:?}", p.guid());

    // let x: Guid = unsafe {
    //     Guid::from_bytes([
    //         0x38, 0x74, 0x77, 0xc2, 0x69, 0xc7, 0x11, 0xd2, 0x8e, 0x39, 0x00,
    // 0xa0, 0xc9, 0x69,         0x72, 0x3b,
    //     ])
    // };
    // println!("{:?}", x);
    // todo!();
}
