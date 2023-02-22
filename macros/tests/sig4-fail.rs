//! Tests that methods fail nicely
use uefi::entry;

struct Embedded {
    //
}

impl Embedded {
    #[entry]
    fn e_main(&self, table: ()) {}
}

fn main() {}
