//! Tests that methods fail nicely
use nuefi::entry;

struct Embedded {
    //
}

impl Embedded {
    #[entry]
    fn e_main(&self, table: ()) {}
}

fn main() {}
