//! Utilities

/// Create a new, transparent, wrapper, around a raw UEFI table or Protocol
/// interface
///
/// Uses a phantom lifetime `'table` to ensure it won't outlive the System Table
///
/// All interfaces derive [`Debug`]
macro_rules! interface {
    ($(
        $(#[$meta:meta])*
        $name:ident($in:ty)
    ),* $(,)*) => {
        $(
            $(#[$meta])*
            #[derive(Debug)]
            #[repr(transparent)]
            pub struct $name<'table> {
                /// Lifetime of this interface is conceptually tied to the [`crate::SystemTable`]
                interface: *mut $in,
                phantom: core::marker::PhantomData<&'table mut $in>,
            }

            impl<'table> $name<'table> {
                /// Create a new interface
                ///
                /// # Safety
                ///
                /// - `interface` must be a valid non-null pointer
                pub(crate) unsafe fn new(interface: *mut $in) -> Self {
                    Self {
                        interface,
                        phantom: core::marker::PhantomData,
                    }
                }

                /// Return a reference to the interface by dereferencing and reborrowing its pointer
                fn interface(&self) -> &$in {
                    // SAFETY:
                    // Ensured valid in construction.
                    // Continued validity ensured by the type system
                    // Should be statically impossible to invalidate
                    unsafe { &*self.interface }
                }
            }
        )*
    };
}

pub(crate) use interface;
