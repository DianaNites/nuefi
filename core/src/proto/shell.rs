//! UEFI Shell Protocol

use self::raw::RawShellParameters;
use crate::{interface, Protocol};

pub mod raw;

interface!(
    #[Protocol("752F3136-4E16-4FDC-A22A-E5F46812F4CA")]
    ShellParameters(RawShellParameters)
);
