// FIXME: TryBuild doest work in miri
#[cfg(not(miri))]
#[test]
fn tests() {
    let t = trybuild::TestCases::new();

    t.pass("tests/*/pass/*.rs");
    t.compile_fail("tests/*/fail/*.rs");
}

// #[cfg(miri)]
