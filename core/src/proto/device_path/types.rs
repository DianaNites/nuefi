//! [`DevicePathHdr`] types
use core::fmt;

#[allow(unused_imports)]
use super::DevicePathHdr;

/// [`DevicePathHdr`] types
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DevicePathType(pub(crate) u8);

impl fmt::Debug for DevicePathType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // f.debug_tuple("DevicePathType").field(&self.0).finish();
        match *self {
            Self::HARDWARE => write!(f, "DevicePathType(Hardware)"),
            Self::ACPI => write!(f, "DevicePathType(Acpi)"),
            Self::MESSAGING => write!(f, "DevicePathType(Messaging)"),
            Self::MEDIA => write!(f, "DevicePathType(Media)"),
            Self::BIOS => write!(f, "DevicePathType(Bios)"),
            Self::END => write!(f, "DevicePathType(End)"),
            _ => f.debug_tuple("DevicePathType").field(&self.0).finish(),
        }
    }
}

impl DevicePathType {
    /// Represents a device connected to the system
    pub const HARDWARE: Self = Self(0x01);

    /// Represents ACPI Plug and Play hardware?
    pub const ACPI: Self = Self(0x02);

    /// Represents the connection to a device on another system,
    /// such as a IP address or SCSI ID.
    pub const MESSAGING: Self = Self(0x03);

    /// Represents the portion of an entity that is being abstracted,
    /// such as a file path on a storage device.
    pub const MEDIA: Self = Self(0x04);

    /// Used by platform firmware to select legacy bios boot options
    pub const BIOS: Self = Self(0x05);

    /// Represents the end of the device path structure or instance
    pub const END: Self = Self(0x7F);
}

/// [`DevicePathHdr`] Sub Types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DevicePathSubType(pub(crate) u8);

pub mod sub {
    use crate::proto::device_path::types::DevicePathSubType;

    pub mod hardware {
        //! Defines how a device is attached to the "resource domain" of the
        //! system, the shared memory, MMIO, and I/O space of the system.
        use super::*;

        /// Path to the PCI configuration space address for a PCI device
        ///
        /// Must be preceded by an [`super::acpi`] entry uniquely
        /// identifying the PCI root bus
        pub const PCI: DevicePathSubType = DevicePathSubType(1);

        pub const PC_CARD: DevicePathSubType = DevicePathSubType(2);

        pub const MEMORY_MAPPED: DevicePathSubType = DevicePathSubType(3);

        pub const VENDOR: DevicePathSubType = DevicePathSubType(4);

        pub const CONTROLLER: DevicePathSubType = DevicePathSubType(5);

        pub const BMC: DevicePathSubType = DevicePathSubType(6);
    }

    pub mod acpi {
        //! Device Paths of this type contain "values that must match exactly
        //! the ACPI name space that is provided by the platform firmware to the
        //! operating system"
        use super::*;

        /// This sub-type only includes the `_HID` and `_UID` fields,
        /// and only as 32-bit numeric values.
        pub const SIMPLE: DevicePathSubType = DevicePathSubType(1);

        /// This sub-type includes the `_HID`, `_CID`, and `_UID` fields,
        /// and supports both numeric and string values.
        pub const EXPANDED: DevicePathSubType = DevicePathSubType(2);

        /// Contains video output device attributes to support the
        /// Graphics Output Protocol.
        ///
        /// Multiple entries may exist if multiple devices are displaying the
        /// same output.
        pub const ADR: DevicePathSubType = DevicePathSubType(3);

        /// Describes an NVDIMM device using the ACPI 6.0
        /// specification defined NFIT Device Handle as the identifier.
        pub const NVDIMM: DevicePathSubType = DevicePathSubType(4);
    }

    pub mod messaging {
        //! Describes the connection of devices outside the
        //! ["resource domain"][super::hardware] of the system
        use super::*;

        pub const ATAPI: DevicePathSubType = DevicePathSubType(1);

        pub const SCSI: DevicePathSubType = DevicePathSubType(2);

        pub const FIBRE: DevicePathSubType = DevicePathSubType(3);

        /// Clarifies the definition of [`FIBRE`] to comply with the
        /// T-10 SCSI Architecture Model 4 specification.
        ///
        /// "The Fibre Channel Ex device path clarifies the definition of the
        /// Logical Unit Number field to conform with the T-10 SCSI Architecture
        /// Model 4 specification. The 8 byte Logical Unit Number field in the
        /// device path must conform with a logical unit number returned by a
        /// SCSI REPORT LUNS command"
        /// UEFI Specification 2.10, section 10.3.4.3. Fibre Channel Device Path
        pub const FIBRE_EX: DevicePathSubType = DevicePathSubType(21);

        #[doc(alias = "1394")]
        pub const FIRE_WIRE: DevicePathSubType = DevicePathSubType(4);

        pub const USB: DevicePathSubType = DevicePathSubType(5);

        pub const SATA: DevicePathSubType = DevicePathSubType(18);

        pub const USB_WWID: DevicePathSubType = DevicePathSubType(16);

        /// Logical Unit Number
        pub const LUN: DevicePathSubType = DevicePathSubType(17);

        pub const USB_CLASS: DevicePathSubType = DevicePathSubType(15);

        pub const I2O: DevicePathSubType = DevicePathSubType(6);

        pub const MAC: DevicePathSubType = DevicePathSubType(11);

        pub const IPV4: DevicePathSubType = DevicePathSubType(12);

        pub const IPV6: DevicePathSubType = DevicePathSubType(13);

        pub const VLAN: DevicePathSubType = DevicePathSubType(20);

        pub const INFINI_BAND: DevicePathSubType = DevicePathSubType(9);

        pub const UART: DevicePathSubType = DevicePathSubType(14);

        // TODO: UEFI defines some vendors
        pub const VENDOR: DevicePathSubType = DevicePathSubType(10);

        pub const SAS_EX: DevicePathSubType = DevicePathSubType(22);

        pub const ISCSI: DevicePathSubType = DevicePathSubType(19);

        pub const NVME: DevicePathSubType = DevicePathSubType(23);

        pub const URI: DevicePathSubType = DevicePathSubType(24);

        pub const UFS: DevicePathSubType = DevicePathSubType(25);

        pub const SD: DevicePathSubType = DevicePathSubType(26);

        pub const BLUETOOTH: DevicePathSubType = DevicePathSubType(27);

        pub const WIFI: DevicePathSubType = DevicePathSubType(28);

        pub const EMMC: DevicePathSubType = DevicePathSubType(29);

        pub const BLUETOOTH_LE: DevicePathSubType = DevicePathSubType(30);

        pub const DNS: DevicePathSubType = DevicePathSubType(31);

        pub const NVDIMM: DevicePathSubType = DevicePathSubType(32);

        pub const REST: DevicePathSubType = DevicePathSubType(33);

        pub const NVME_FABRIC: DevicePathSubType = DevicePathSubType(34);
    }

    pub mod media {
        //! Describes the portion of a medium that is being abstracted
        //! by a boot service
        use super::*;

        /// A partition on a hard drive
        pub const HARD_DRIVE: DevicePathSubType = DevicePathSubType(1);

        /// A partition on a CD-ROM
        pub const CD_ROM: DevicePathSubType = DevicePathSubType(2);

        pub const VENDOR: DevicePathSubType = DevicePathSubType(3);

        pub const FILE: DevicePathSubType = DevicePathSubType(4);

        pub const MEDIA: DevicePathSubType = DevicePathSubType(5);

        /// UEFI PI Specification firmware file
        pub const FIRMWARE_FILE: DevicePathSubType = DevicePathSubType(6);

        /// UEFI PI Specification firmware volume
        pub const FIRMWARE_VOLUME: DevicePathSubType = DevicePathSubType(7);

        /// Offset relative from the *start* of the device
        pub const RELATIVE_OFFSET: DevicePathSubType = DevicePathSubType(8);

        pub const RAM_DISK: DevicePathSubType = DevicePathSubType(9);
    }

    pub mod bios {
        //! Describe the booting of non-EFI-aware systems
        //!
        //! Only required to allow booting non-EFI systems
        use super::*;

        pub const BIOS: DevicePathSubType = DevicePathSubType(1);
    }
}

impl DevicePathSubType {
    /// Represents the end of the entire [`DevicePathHdr`]
    pub const END_ENTIRE: Self = Self(0xFF);

    /// Represents the end of this [`DevicePathHdr`] instance
    /// and the start of a new one
    pub const END_INSTANCE: Self = Self(0x01);
}
