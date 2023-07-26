//! UEFI Tables

use alloc::{string::String, vec::Vec};
use core::{
    ffi::c_void,
    iter::from_fn,
    marker::PhantomData,
    mem::{size_of, transmute, MaybeUninit},
    ptr::{null_mut, NonNull},
    slice::{from_raw_parts, from_raw_parts_mut},
    time::Duration,
};

use nuefi_core::interface;
pub use nuefi_core::table::config;

use crate::{
    error::{Result, Status},
    get_image_handle,
    mem::MemoryType,
    proto::{
        self,
        console::SimpleTextOutput,
        device_path::{raw::RawDevicePath, DevicePath},
        Guid,
        Protocol,
        Scope,
    },
    string::{UefiStr, UefiString},
    EfiHandle,
};

pub mod raw {
    // FIXME: Imports
    pub use nuefi_core::table::{
        boot_fn::*,
        config::ConfigurationTable as RawConfigurationTable,
        BootServices as RawBootServices,
        Header,
        LocateSearch,
        Revision,
        RuntimeServices as RawRuntimeServices,
        SystemTable as RawSystemTable,
    };
}
use raw::*;

mod boot;
pub use boot::BootServices;

mod runtime;
pub use runtime::RuntimeServices;

/// Type marker for [`SystemTable`] representing before ExitBootServices is
/// called
pub struct Boot;

/// Type marker for [`SystemTable`] representing after ExitBootServices is
/// called
pub struct Runtime;

/// Internal state for global handling code
pub(crate) struct Internal;

/// The UEFI System table
///
/// This is your entry-point to using UEFI and all its services
// NOTE: This CANNOT be Copy or Clone, as this would violate the planned
// safety guarantees of passing it to ExitBootServices.
//
// It is also important that the lifetimes involved stay within their
// respective structures, that the lifetime of the SystemTable is not used
// for data from BootServices.
// That way we can potentially statically prevent incorrect ExitBootServices
// calls, without invalidating RuntimeServices?
// Existing RuntimeServices and pointers might become invalid though?
//
// Defining lifetimes this way should be fine either way though
//
// --
//
// The idea around the design and safety of this structure is that
// the only safe way to obtain an instance of this structure is get it from
// your entry point, after its been validated by our entry point.
//
// Ownership of this value represents access to the table resource,
// and lifetimes are derived from the lifetime of this owned value passed to.
// The SystemTable is mutable and can potentially be changed between uses,
// so no long term references are created to it.
//
// Instead, this is a wrapper around the *pointer* to Table, and performs
// all access anew on-demand. Note that this pointer is a physical pointer, not
// virtual.
//
// This table is notably modified by firmware when ExitBootServices is called,
// some fields become invalid, and all memory not of type
// [`MemoryType::RUNTIME_*`] is deallocated.
// The [`BootServices`] table, and all protocols, become invalid.
//
// The system table is still valid after this call, and we now own all memory.
//
// The lifetime of this table is technically valid for the rest of the life of
// the system, all the way until shutdown, with the above caveats, though its
// pointer is not stable.
//
// We use type states and lifetimes derived from ownership of this value to
// attempt to encode this logic.
//
// In the [`Boot`] state, it is valid to use [`BootServices`] and
// [`RuntimeServices`] at the same time.
//
// In the [`Runtime`] state, it is only valid to use [`RuntimeServices`] and
// the fields specified by [`RawSystemTable`]
#[derive(Debug)]
#[repr(transparent)]
pub struct SystemTable<State> {
    /// Pointer to the table.
    ///
    /// Conceptually, this is static, it will be alive for the life of the
    /// program.
    ///
    /// That said, it would be problematic if this was a static reference,
    /// because it can change out from under us, such as when ExitBootServices
    /// is called.
    table: *mut RawSystemTable,

    phantom: PhantomData<*mut State>,
}

// Internal, all
impl<T> SystemTable<T> {
    /// Create new SystemTable
    ///
    /// # Safety
    ///
    /// - `this` must be a valid `RawSystemTable`
    /// - `this` must have been validated [`RawSystemTable::validate`]
    pub(crate) unsafe fn new(this: *mut RawSystemTable) -> Self {
        Self {
            table: this,
            phantom: PhantomData,
        }
    }

    fn table(&self) -> &RawSystemTable {
        // Safety:
        // - The existence of `&self` implies this pointer is valid
        // - The system table pointer will remain unless we remap its address
        // - Remapping is not currently implemented, so it cannot safely be done.
        unsafe { &*self.table }
    }
}

// Internal, all
impl SystemTable<Internal> {
    /// Get the SystemTable if still in boot mode.
    ///
    /// This is useful for the logging, panic, and alloc error handlers
    ///
    /// If ExitBootServices has NOT been called,
    /// return [`SystemTable<Boot>`], otherwise [`None`]
    pub(crate) fn as_boot(&self) -> Option<SystemTable<Boot>> {
        if !self.table().boot_services.is_null() {
            // Safety:
            // - Above check verifies ExitBootServices has not been called.
            Some(unsafe { SystemTable::new(self.table) })
        } else {
            None
        }
    }
}

/// Available during Boot Services
impl SystemTable<Boot> {
    /// String identifying the firmware vendor
    pub fn firmware_vendor(&self) -> UefiStr<'_> {
        let p = self.table().firmware_vendor;
        debug_assert!(!p.is_null(), "firmware vendor was null");
        // Safety: UEFI firmware responsibility
        unsafe { UefiStr::from_ptr(p) }
    }

    /// Firmware-specific value indicating its revision
    pub fn firmware_revision(&self) -> u32 {
        self.table().firmware_revision
    }

    /// Returns the UEFI [`Revision`] that this implementation claims
    /// conformance to
    pub fn uefi_revision(&self) -> Revision {
        self.table().header.revision
    }

    /// A copy of the UEFI Table header structure
    pub fn header(&self) -> Header {
        self.table().header
    }

    /// Output on stdout.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn stdout(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_out;
        assert!(!ptr.is_null(), "con_out handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr.cast()) }
    }

    /// Output on stderr.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn stderr(&self) -> SimpleTextOutput<'_> {
        let ptr = self.table().con_err;
        assert!(!ptr.is_null(), "std_err handle was null");
        // Safety: Construction ensures safety.
        unsafe { SimpleTextOutput::new(ptr.cast()) }
    }

    /// Reference to the UEFI Boot services.
    ///
    /// This is only valid for as long as the SystemTable is
    pub fn boot(&self) -> BootServices<'_> {
        let ptr = self.table().boot_services;
        assert!(!ptr.is_null(), "boot_services handle was null");
        // Safety: Construction ensures safety.
        unsafe { BootServices::new(ptr) }
    }

    /// Iterator over UEFI Configuration tables
    ///
    /// See [`config`] and [`config::GenericConfig`] for details
    pub fn config_tables(&self) -> impl Iterator<Item = config::GenericConfig<'_>> + '_ {
        let data = self.table().configuration_table;
        let len = self.table().number_of_table_entries;
        assert!(!data.is_null(), "UEFI Configuration table pointer was null");

        // Safety: The pointer is valid for this many elements according
        // to the UEFI spec
        // The returned lifetime will be tied to `self`, which is valid.
        let tables = unsafe { from_raw_parts(data, len).iter().copied() };

        tables.map(config::GenericConfig::new)
    }

    /// Get the configuration table specified by `T`, or [`None`]
    ///
    /// See [`config`] and [`config::ConfigTable`] for details
    pub fn config_table<'tbl, T: config::ConfigTable<'tbl>>(&'tbl self) -> Option<T::Out<'tbl>>
    where
        Self: 'tbl,
    {
        self.config_tables()
            .find(|t| t.guid() == T::GUID)
            .and_then(|t| t.as_table::<T>())
    }
}
