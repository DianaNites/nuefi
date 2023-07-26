use nuefi_core::interface;

interface!(
    /// The UEFI Runtime Services
    RuntimeServices(nuefi_core::table::RuntimeServices),
);
