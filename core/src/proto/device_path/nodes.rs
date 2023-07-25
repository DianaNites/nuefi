//! Specific Device Path nodes
#![allow(unused_imports, dead_code, unused_macros)]
use super::{types::*, DevicePathHdr};
use crate::base::Guid;

/// End Of Device Path node
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct End {
    hdr: DevicePathHdr,
}

impl End {
    /// Mark the end of the entire device path
    pub const fn entire() -> Self {
        Self {
            hdr: DevicePathHdr {
                ty: DevicePathType::END,
                sub_ty: DevicePathSubType::END_ENTIRE,
                len: 4u16.to_le_bytes(),
            },
        }
    }

    /// Mark the end of an instance of a device path in a multi-instance path
    pub const fn instance() -> Self {
        Self {
            hdr: DevicePathHdr {
                ty: DevicePathType::END,
                sub_ty: DevicePathSubType::END_INSTANCE,
                len: 4u16.to_le_bytes(),
            },
        }
    }
}

pub mod hardware {
    //! Defines how a device is attached to the "resource domain" of the
    //! system, the shared memory, MMIO, and I/O space of the system.
    use super::*;

    /// Path to the PCI configuration space address for a PCI device
    ///
    /// Must be preceded by an [`super::acpi`] entry uniquely
    /// identifying the PCI root bus
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct PciPath {
        hdr: DevicePathHdr,
        function: u8,
        device: u8,
    }

    /// PC Card device
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct PcCard {
        hdr: DevicePathHdr,
        function: u8,
    }

    /// Memory Mapped device
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct MemoryMapped {
        hdr: DevicePathHdr,
        mem_ty: [u8; 4],
        start: [u8; 8],
        end: [u8; 8],
    }

    /// Vendor defined path, contents defined by the vendor GUID
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Vendor {
        hdr: DevicePathHdr,
        guid: [u8; 16],
        data: [u8; 0],
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Controller {
        hdr: DevicePathHdr,
        num: [u8; 4],
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Bmc {
        hdr: DevicePathHdr,
        interface: u8,
        base: [u8; 8],
    }

    //

    impl PciPath {
        pub const fn new(function: u8, device: u8) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::PCI,
                    len: 6u16.to_le_bytes(),
                },
                function,
                device,
            }
        }
    }

    impl PcCard {
        pub const fn new(function: u8) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::PC_CARD,
                    len: 5u16.to_le_bytes(),
                },
                function,
            }
        }
    }

    impl MemoryMapped {
        pub const fn new(mem_ty: u32, start: usize, end: usize) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::MEMORY_MAPPED,
                    len: 24u16.to_le_bytes(),
                },

                mem_ty: mem_ty.to_le_bytes(),
                start: start.to_le_bytes(),
                end: end.to_le_bytes(),
            }
        }
    }

    impl Vendor {
        /// Write the header of a Vendor node expecting `data_len` bytes to
        /// follow.
        ///
        /// # Panics
        ///
        /// If `data_len + 20` would overflow.
        pub const fn new_header(guid: Guid, data_len: u16) -> Self {
            let len = {
                let this = data_len.checked_add(20);
                match this {
                    Some(val) => val,
                    None => panic!("data_len + 20 overflowed"),
                }
            };

            let len = len.to_le_bytes();
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::VENDOR,
                    len,
                },
                guid: guid.to_bytes(),
                data: [],
            }
        }
    }

    impl Controller {
        pub const fn new(num: u32) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::CONTROLLER,
                    len: 8u16.to_le_bytes(),
                },
                num: num.to_le_bytes(),
            }
        }
    }

    impl Bmc {
        pub const fn new(interface: u8, base: usize) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::HARDWARE,
                    sub_ty: sub::hardware::BMC,
                    len: 13u16.to_le_bytes(),
                },
                interface,
                base: base.to_le_bytes(),
            }
        }
    }
}

pub mod acpi {
    //! Device Paths of this type contain "values that must match exactly
    //! the ACPI name space that is provided by the platform firmware to the
    //! operating system"
    use super::*;
    // TODO: Figure out the EFI_PNP_ID macro and etc

    /// Acpi Simple
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Acpi {
        hdr: DevicePathHdr,

        /// ACPI _HID, PnP Hardware ID
        ///
        /// Stored in a 32-bit compressed EISA-type ID.
        /// Incompatible with the ACPI representation.
        hid: [u8; 4],

        /// ACPI _UID, Unique ID between devices with matching `_HID`
        uid: [u8; 4],
    }

    /// ACPI Extended
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct AcpiEx {
        hdr: DevicePathHdr,

        /// ACPI _HID, PnP Hardware ID
        ///
        /// Stored in a 32-bit compressed EISA-type ID.
        /// Incompatible with the ACPI representation.
        hid: [u8; 4],

        /// ACPI _UID, Unique ID between devices with matching `_HID`
        uid: [u8; 4],

        /// ACPI _CID
        ///
        /// Stored in a 32-bit compressed EISA-type ID.
        /// Incompatible with the ACPI representation.
        cid: [u8; 4],

        hid_str: [u8; 0],
        uid_str: [u8; 0],
        cid_str: [u8; 0],
    }

    /// Contains video output device attributes to support the
    /// Graphics Output Protocol.
    ///
    /// Multiple entries may exist if multiple devices are displaying the
    /// same output.
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Adr {
        hdr: DevicePathHdr,

        /// ACPI _HID, PnP Hardware ID
        ///
        /// Stored in a 32-bit compressed EISA-type ID.
        /// Incompatible with the ACPI representation.
        adr: [u8; 4],

        extra: [u8; 0],
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Nvdimm {
        hdr: DevicePathHdr,

        /// NFIT Device Handle
        handle: [u8; 4],
    }

    //

    impl Acpi {
        const fn new(hid: u32, uid: u32) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::ACPI,
                    sub_ty: sub::acpi::SIMPLE,
                    len: 12u16.to_le_bytes(),
                },
                hid: hid.to_le_bytes(),
                uid: uid.to_le_bytes(),
            }
        }
    }

    // TODO: AcpiEx
    #[cfg(no)]
    impl AcpiEx {
        /// Write the header of a [`AcpiEx`]
        pub const fn new_header() -> Self {
            let len: u16 = 19;

            let len = len.to_le_bytes();
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::ACPI,
                    sub_ty: sub::acpi::EXPANDED,
                    len,
                },
            }
        }
    }

    impl Adr {
        /// Write the header of an [`Adr`] expecting to be followed by
        /// `entries` additional *4 bytes* `adr` values.
        ///
        /// # Panics
        ///
        /// If `entries` would overflow `u16`
        const fn new_header(adr: u32, entries: u16) -> Self {
            let len: u16 = {
                let this = 8u16.checked_add({
                    let this = entries.checked_mul(4);
                    match this {
                        Some(val) => val,
                        None => panic!("`entries * 4` overflowed"),
                    }
                });
                match this {
                    Some(val) => val,
                    None => panic!("`8 + (entries * 4)` overflowed"),
                }
            };

            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::ACPI,
                    sub_ty: sub::acpi::ADR,
                    len: len.to_le_bytes(),
                },
                adr: adr.to_le_bytes(),
                extra: [],
            }
        }

        /// Create a new entry with one adr
        const fn new_one(adr: u32) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::ACPI,
                    sub_ty: sub::acpi::ADR,
                    len: 8u16.to_le_bytes(),
                },
                adr: adr.to_le_bytes(),
                extra: [],
            }
        }
    }

    impl Nvdimm {
        const fn new(handle: u32) -> Self {
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::ACPI,
                    sub_ty: sub::acpi::NVDIMM,
                    len: 8u16.to_le_bytes(),
                },
                handle: handle.to_le_bytes(),
            }
        }
    }
}

pub mod media {
    use super::*;

    /// Vendor defined path, contents defined by the vendor GUID
    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct Vendor {
        hdr: DevicePathHdr,
        guid: [u8; 16],
        data: [u8; 0],
    }

    #[derive(Debug, Clone, Copy)]
    #[repr(C, packed)]
    pub struct File {
        hdr: DevicePathHdr,

        /// Variable length null-terminated path name
        path: [u8; 0],
    }

    //

    impl Vendor {
        /// Write the header of a Vendor node expecting `data_len` bytes to
        /// follow.
        ///
        /// # Panics
        ///
        /// If `data_len + 20` would overflow.
        pub const fn new_header(guid: Guid, data_len: u16) -> Self {
            let len = {
                let this = data_len.checked_add(20);
                match this {
                    Some(val) => val,
                    None => panic!("data_len + 20 overflowed"),
                }
            };

            let len = len.to_le_bytes();
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::MEDIA,
                    sub_ty: sub::hardware::VENDOR,
                    len,
                },
                guid: guid.to_bytes(),
                data: [],
            }
        }
    }

    impl File {
        /// Write a [`File`] header expecting `len` bytes to follow
        ///
        /// # Panics
        ///
        /// If `len` would overflow `u16`.
        pub const fn new_header(len: u16) -> Self {
            let len = {
                let this = len.checked_add(4);
                match this {
                    Some(val) => val,
                    None => panic!("File::new_header `len + 4` overflowed"),
                }
            };
            Self {
                hdr: DevicePathHdr {
                    ty: DevicePathType::MEDIA,
                    sub_ty: sub::media::FILE,
                    len: len.to_le_bytes(),
                },
                path: [],
            }
        }
    }
}
