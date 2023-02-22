//! Test that an obviously invalid GUID fails
use uefi::Protocol;

// Random UUID from `uuidgen` with 69420 added to it
const GUID: &str = "c986ec27-69420-af54-4b55-80aa-91697fcdf8eb";

#[repr(C)]
struct RawProto {
    pro: *mut RawProto,
}

#[Protocol("c986ec27-69420-af54-4b55-80aa-91697fcdf8eb")]
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

fn main() {}
