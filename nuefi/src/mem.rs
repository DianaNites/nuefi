//! UEFI Boot time allocator
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use crate::get_boot_table;

/// UEFI always aligns to 8.
const POOL_ALIGN: usize = 8;

pub use nuefi_core::table::mem::{
    AllocateType,
    MemoryDescriptor,
    MemoryFlags,
    MemoryType,
    PhysicalAddress,
    VirtualAddress,
};

/// A UEFI memory allocator
///
/// Relies on [`BootServices::allocate_pool`][allocate_pool]
/// and [`BootServices::free_pool`][free_pool].
///
/// Allocates all data in [`MemoryType::LOADER_DATA`]
///
/// After ExitBootServices is called, all allocations will fail.
///
/// [allocate_pool]: crate::table::BootServices::allocate_pool
/// [free_pool]: crate::table::BootServices::free_pool
pub struct UefiAlloc {
    _priv: (),
}

impl UefiAlloc {
    pub const fn new() -> Self {
        Self { _priv: () }
    }
}

// Safety: We adhere to the contract of GlobalAlloc
unsafe impl GlobalAlloc for UefiAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // trace!("UEFI allocating {layout:?}");

        let align = layout.align();
        let size = layout.size();
        let offset = if align > POOL_ALIGN {
            let o = align - POOL_ALIGN;
            // trace!(
            //"Allocation alignment {align} greater than {POOL_ALIGN}, using {} as offset",
            //     o
            // );
            o
        } else {
            0
        };
        let size = size + offset;

        if let Some(table) = get_boot_table() {
            let ret = table.boot().allocate_pool(MemoryType::LOADER_DATA, size);
            if let Ok(ptr) = ret {
                let ptr = ptr.as_ptr();
                // trace!(
                //     "Old pointer {ptr:p} vs new pointer {:p} (aligned: {})",
                //     ptr.add(offset),
                //     ptr as usize & (offset.saturating_sub(1)) == 0
                // );
                ptr.add(offset).cast()
            } else {
                null_mut()
            }
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let align = layout.align();
        let _size = layout.size();
        let offset = if align > POOL_ALIGN {
            let o = align - POOL_ALIGN;
            // trace!(
            //"Deallocation alignment {align} greater than {POOL_ALIGN}, using {} as offset",
            //     o
            // );
            o
        } else {
            0
        };
        let _size = _size + offset;

        if let Some(table) = get_boot_table() {
            let ptr = ptr.sub(offset);
            let ret = table.boot().free_pool(ptr.cast());
            if let Err(e) = ret {
                // error!("Error {e} while deallocating memory {ptr:p} with
                // layout {layout:?}");
            }
        }
    }
}

// Safety: Synchronized by UEFI? UEFI has one thread, and we're it.
unsafe impl Sync for UefiAlloc {}
