//! UEFI Memory allocation related types

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MemoryType(u32);

impl MemoryType {
    pub const RESERVED: Self = Self(0);

    /// UEFI Application code
    pub const LOADER_CODE: Self = Self(1);

    /// UEFI Application data
    pub const LOADER_DATA: Self = Self(2);

    /// UEFI Boot Service code
    pub const BOOT_CODE: Self = Self(3);

    /// UEFI Boot Service data
    pub const BOOT_DATA: Self = Self(4);

    /// UEFI Runtime Service code
    pub const RUNTIME_CODE: Self = Self(5);

    /// UEFI Runtime Service data
    pub const RUNTIME_DATA: Self = Self(6);

    /// Free / unallocated memory
    pub const CONVENTIONAL: Self = Self(7);

    /// Memory with errors
    pub const UNUSABLE: Self = Self(8);

    /// Holds ACPI Tables
    pub const ACPI_RECLAIM: Self = Self(9);

    /// Reserved by firmware
    pub const ACPI_NVS: Self = Self(10);

    /// Used by firmware to request a memory mapped IO region from the OS,
    /// for Runtime Services
    pub const MEMORY_MAPPED_IO: Self = Self(11);

    /// System memory-mapped IO region that is used to translate memory cycles
    /// to IO cycles by the processor.
    pub const MEMORY_MAPPED_IO_PORTS: Self = Self(12);

    /// Reserved by firmware for code that is part of the processor
    pub const PAL: Self = Self(13);

    /// The same as [`CONVENTIONAL`][`MemoryType::CONVENTIONAL`],
    /// except happens to be persistent.
    pub const PERSISTENT: Self = Self(14);

    /// Must be accepted by the boot target before it becomes usable
    pub const UNACCEPTED: Self = Self(15);

    /// Max value.
    const _MAX: Self = Self(16);
}

/// UEFI Memory flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
