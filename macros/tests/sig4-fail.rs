//! Tests that methods fail nicely
struct Embedded {
    //
}

impl Embedded {
    #[macros::entry]
    fn e_main(&self, table: ()) {}
}

fn main() {}
