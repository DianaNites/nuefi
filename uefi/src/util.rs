//! Utilities

use core::marker::PhantomData;

/// The UEFI Boot services
#[repr(transparent)]
pub struct PointerWrapper<'table, Interface> {
    /// Lifetime conceptually tied to [`crate::SystemTable`]
    interface: *mut Interface,
    phantom: PhantomData<&'table mut Interface>,
}

impl<'table, Interface> PointerWrapper<'table, Interface> {
    pub fn new(interface: *mut Interface) -> Self {
        Self {
            interface,
            phantom: PhantomData,
        }
    }
}
