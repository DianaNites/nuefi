//! UEFI Boot time allocator
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use log::{error, trace};

use crate::get_boot_table;

/// UEFI always aligns to 8.
const POOL_ALIGN: usize = 8;

/// UEFI Physical Address
#[repr(transparent)]
pub struct PhysicalAddress(u64);

/// UEFI Virtual Address
#[repr(transparent)]
pub struct VirtualAddress(u64);

/// UEFI Allocation type
#[repr(transparent)]
pub struct AllocateType(u32);

impl AllocateType {
    /// Allocate any available range that satisfies request
    pub const ANY_PAGES: Self = Self(0);

    /// Allocate range whose uppermost address is less than or equal to the
    /// input
    pub const MAX_ADDRESS: Self = Self(1);

    /// Allocate to this physical address
    pub const ADDRESS: Self = Self(2);

    /// Max value.
    const _MAX: Self = Self(3);
}

/// UEFI Memory type
#[derive(Debug)]
#[repr(transparent)]
pub struct MemoryType(u32);

impl MemoryType {
    pub const RESERVED: Self = Self(0);
    pub const LOADER_CODE: Self = Self(1);
    pub const LOADER_DATA: Self = Self(2);
    pub const BOOT_CODE: Self = Self(3);
    pub const BOOT_DATA: Self = Self(4);
    pub const RUNTIME_CODE: Self = Self(5);
    pub const RUNTIME_DATA: Self = Self(6);
    pub const CONVENTIONAL: Self = Self(7);
    pub const UNUSABLE: Self = Self(8);
    pub const ACPI_RECLAIM: Self = Self(9);
    pub const ACPI_NVS: Self = Self(10);
    pub const MEMORY_MAPPED_IO: Self = Self(11);
    pub const MEMORY_MAPPED_IO_PORTS: Self = Self(12);
    pub const PAL: Self = Self(13);
    pub const PERSISTENT: Self = Self(14);
    pub const UNACCEPTED: Self = Self(15);

    /// Max value.
    const _MAX: Self = Self(16);
}

/// UEFI Memory flags
#[repr(transparent)]
pub struct MemoryFlags(u64);

impl MemoryFlags {
    pub const UC: Self = Self(0x0000000000000001);
    pub const WC: Self = Self(0x0000000000000002);
    pub const WT: Self = Self(0x0000000000000004);
    pub const WB: Self = Self(0x0000000000000008);
    pub const UCE: Self = Self(0x0000000000000010);
    pub const WP: Self = Self(0x0000000000001000);
    pub const RP: Self = Self(0x0000000000002000);
    pub const XP: Self = Self(0x0000000000004000);
    pub const NV: Self = Self(0x0000000000008000);
    pub const MORE_RELIABLE: Self = Self(0x0000000000010000);
    pub const RO: Self = Self(0x0000000000020000);
    pub const SP: Self = Self(0x0000000000040000);
    pub const CPU_CRYPTO: Self = Self(0x0000000000080000);
    pub const RUNTIME: Self = Self(0x8000000000000000);
    pub const ISA_VALID: Self = Self(0x4000000000000000);
    pub const ISA_MASK: Self = Self(0x0FFFF00000000000);
}

/// UEFI Memory Descriptor
#[repr(C)]
pub struct MemoryDescriptor {
    ty: MemoryType,
    start: PhysicalAddress,
    pages: u64,
    attribute: MemoryFlags,
}

impl MemoryDescriptor {
    pub(crate) const _VERSION: u32 = 1;
}

/// A UEFI memory allocator
///
/// After ExitBootServices is called, all allocations will fail.
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
        trace!("UEFI allocating {layout:?}");

        let align = layout.align();
        let size = layout.size();
        let offset = if align > POOL_ALIGN {
            let o = align - POOL_ALIGN;
            trace!(
                "Allocation alignment {align} greater than {POOL_ALIGN}, using {} as offset",
                o
            );
            o
        } else {
            0
        };
        let size = size + offset;

        if let Some(table) = get_boot_table() {
            let ret = table.boot().allocate_pool(MemoryType::LOADER_DATA, size);
            if let Ok(ptr) = ret {
                trace!(
                    "Old pointer {ptr:p} vs new pointer {:p} (aligned: {})",
                    ptr.add(offset),
                    ptr as usize & (offset.saturating_sub(1)) == 0
                );
                ptr.add(offset)
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
            trace!(
                "Deallocation alignment {align} greater than {POOL_ALIGN}, using {} as offset",
                o
            );
            o
        } else {
            0
        };
        let _size = _size + offset;

        if let Some(table) = get_boot_table() {
            let ptr = ptr.sub(offset);
            let ret = table.boot().free_pool(ptr);
            if let Err(e) = ret {
                error!("Error {e} while deallocating memory {ptr:p} with layout {layout:?}");
            }
        }
    }
}

// Safety: Synchronized by UEFI? UEFI has one thread, and we're it.
unsafe impl Sync for UefiAlloc {}
