//! UEFI Media protocols
use raw::RawLoadFile2;

use crate::{
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
