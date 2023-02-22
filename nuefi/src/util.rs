//! Utilities

/// Create a new, transparent, wrapper, around a raw UEFI table or Protocol
/// interface
///
/// Uses a phantom lifetime `'table` to ensure it won't outlive the System Table
///
/// All interfaces derive [`Debug`]
///
/// # Safety
///
/// - You must be a developer of this library
#[macro_export]
#[doc(hidden)]
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

            // Its okay not to use these macro generated interfaces, shut up
            #[allow(dead_code)]
            impl<'table> $name<'table> {
                /// Create a new interface
                ///
                /// # Safety
                ///
                /// - `interface` must be a valid non-null pointer
                /// - Only called from [crate::SystemTable] or [crate::proto::Protocol::from_raw]
                /// - Or "simple" getters from a protocol
                ///
                /// Be VERY CAREFUL about the lifetime this synthesizes,
                /// or else it will be possible to live longer than it should and cause UB.
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

                /// Return a mutable reference to the interface by dereferencing and reborrowing its pointer
                #[allow(clippy::mut_from_ref)]
                fn interface_mut(&self) -> &mut $in {
                    // SAFETY:
                    // Ensured valid in construction.
                    // Continued validity ensured by the type system
                    // Should be statically impossible to invalidate
                    unsafe { &mut *self.interface }
                }

                /// Raw pointer to this protocols interface
                ///
                /// It is your responsibility to use it correctly.
                pub fn as_ptr(&self) -> *mut $in {
                    self.interface
                }
            }
        )*
    };
}

pub(crate) use interface;
