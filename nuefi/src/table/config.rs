//! UEFI Configuration Tables
//!
//! UEFI Configuration tables are entries in an array within the
//! [`crate::SystemTable`].
//!
//! They are completely arbitrary data, identified by and whose type is
//! determined by a [`Guid`].
//!
//! Several standard and vendor-specific tables are defined and known about
//! here. Unknown tables can be used through [`GenericConfig`]
use super::*;
use crate::{proto::Entity, GUID};

mod imp {
    use super::*;
    pub trait Sealed {}
    impl Sealed for AcpiTable10 {}
    impl Sealed for AcpiTable20 {}
    impl Sealed for RuntimeProperties {}
    impl Sealed for SMBIOS {}
    impl Sealed for SMBIOS3 {}
    impl Sealed for SAL {}
    impl Sealed for MPS {}
    impl Sealed for JsonConfigData {}
    impl Sealed for JsonCapsuleData {}
    impl Sealed for JsonCapsuleResult {}
    impl Sealed for DeviceTree {}
    impl Sealed for MemoryAttributes {}
    impl Sealed for ConformanceProfile {}
}

pub mod vendor {
    //! Vendor Specific Configuration tables

    pub mod edk2 {
        //! Configuration tables specific to the common TianoCore EDK2 UEFI
        //! Implementation.
        use crate::GUID;

        #[GUID("A31280AD-481E-41B6-95E8-127F4C984779", crate("crate"))]
        #[derive(Debug)]
        #[repr(C)]
        pub struct TianoCompress {
            table: *mut u8,
        }

        #[GUID("EE4E5898-3914-4259-9D6E-DC7BD79403CF", crate("crate"))]
        #[derive(Debug)]
        #[repr(C)]
        pub struct LZMACompress {
            table: *mut u8,
        }

        #[GUID("3D532050-5CDA-4FD0-879E-0F7F630D5AFB", crate("crate"))]
        #[derive(Debug)]
        #[repr(C)]
        pub struct BrotliCompress {
            table: *mut u8,
        }

        #[GUID("D42AE6BD-1352-4bfb-909A-CA72A6EAE889", crate("crate"))]
        #[derive(Debug)]
        #[repr(C)]
        pub struct LZMAf86Compress {
            table: *mut u8,
        }
    }
}
use vendor::edk2::*;

/// Identifies a UEFI Configuration Table
///
/// Specifically, this is a sealed trait that identifies a table definition
/// we statically *trust*. This has safety implications, an incorrect GUID
/// can result in type confusion and thus unsoundness.
///
/// Any type that implements this is thus guaranteed to be sound.
///
/// [`GenericConfig`] exposes a generic method for unsafely handling arbitrary
/// configuration tables if you need this.
// `'tbl` represents the lifetime of the [`SystemTable`]
pub trait ConfigTable<'tbl>: Entity + imp::Sealed {
    /// The lifetime `'cfg` represents the configuration table
    ///
    /// FIXME: I dont really get this honestly.
    type Out<'cfg>
    where
        'tbl: 'cfg;

    /// # Safety
    ///
    /// - `raw` must be valid for this table
    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl>;
}

/// A generic UEFI configuration table, identified by a [`Guid`]
#[derive(Debug)]
#[repr(transparent)]
pub struct GenericConfig<'tbl> {
    config: RawConfigurationTable,

    /// Lifetime of the [`SystemTable`]. All our data is valid for this long.
    phantom: core::marker::PhantomData<&'tbl mut ()>,
}

impl<'tbl> GenericConfig<'tbl> {
    pub(crate) fn new(config: RawConfigurationTable) -> Self {
        Self {
            config,
            phantom: PhantomData,
        }
    }

    /// GUID for the table
    pub fn guid(&self) -> Guid {
        self.config.guid
    }

    /// Raw untyped pointer to the table
    pub fn as_ptr(&self) -> *mut u8 {
        self.config.table
    }

    /// Name of this table, if known
    pub fn name(&self) -> Option<&'static str> {
        // NOTE: Manually keep up to date.
        // TODO: Find better way?
        let guid = self.guid();
        if guid == AcpiTable20::GUID {
            Some(AcpiTable20::NAME)
        } else if guid == AcpiTable10::GUID {
            Some(AcpiTable10::NAME)
        } else if guid == RuntimeProperties::GUID {
            Some(RuntimeProperties::NAME)
        } else if guid == SMBIOS::GUID {
            Some(SMBIOS::NAME)
        } else if guid == SMBIOS3::GUID {
            Some(SMBIOS3::NAME)
        } else if guid == SAL::GUID {
            Some(SAL::NAME)
        } else if guid == MPS::GUID {
            Some(MPS::NAME)
        } else if guid == JsonConfigData::GUID {
            Some(JsonConfigData::NAME)
        } else if guid == JsonCapsuleData::GUID {
            Some(JsonCapsuleData::NAME)
        } else if guid == JsonCapsuleResult::GUID {
            Some(JsonCapsuleResult::NAME)
        } else if guid == DeviceTree::GUID {
            Some(DeviceTree::NAME)
        } else if guid == MemoryAttributes::GUID {
            Some(MemoryAttributes::NAME)
        } else if guid == ConformanceProfile::GUID {
            Some(ConformanceProfile::NAME)
        } else if guid == DebugImageInfo::GUID {
            Some(DebugImageInfo::NAME)
        } else if guid == ImageExecInfo::GUID {
            Some(ImageExecInfo::NAME)
        } else if guid == SystemResource::GUID {
            Some(SystemResource::NAME)
        } else if guid == MemoryRangeCapsule::GUID {
            Some(MemoryRangeCapsule::NAME)
        } else if guid == UserInformation::GUID {
            Some(UserInformation::NAME)
        } else if guid == HIIDatabaseExport::GUID {
            Some(HIIDatabaseExport::NAME)
        } else if guid == EfiProperties::GUID {
            Some(EfiProperties::NAME)
        } else if guid == TianoCompress::GUID {
            Some(TianoCompress::NAME)
        } else if guid == LZMACompress::GUID {
            Some(LZMACompress::NAME)
        } else if guid == BrotliCompress::GUID {
            Some(BrotliCompress::NAME)
        } else if guid == LZMAf86Compress::GUID {
            Some(LZMAf86Compress::NAME)
        } else if guid == DXEServices::GUID {
            Some(DXEServices::NAME)
        } else if guid == HOBlist::GUID {
            Some(HOBlist::NAME)
        } else if guid == MemoryTypeInfo::GUID {
            Some(MemoryTypeInfo::NAME)
        } else if guid == MemoryStatus::GUID {
            Some(MemoryStatus::NAME)
        } else {
            None
        }
    }

    /// If this generic table is [`ConfigTable`] `T`,
    /// then return its typed value.
    /// See the specific table for details
    // This lives as long as `'tbl`, which can only come from
    // the [`SystemTable::config_tables`].
    pub fn as_table<T: ConfigTable<'tbl>>(&self) -> Option<T::Out<'tbl>> {
        if self.guid() == T::GUID {
            let raw = self.as_ptr();
            // Safety: We've just verified the GUID is correct
            // `ConfigTable` is sealed and its trusted to have correct GUIDs and types
            let o = unsafe { T::from_raw(raw) };
            Some(o)
        } else {
            None
        }
    }
}

/// Table for ACPI 2.0 and newer
#[GUID("8868E871-E4F1-11D3-BC22-0080C73C8881", crate("crate"))]
#[derive(Debug)]
pub struct AcpiTable20 {
    table: *mut u8,
}

impl AcpiTable20 {
    #[inline]
    pub const fn table(&self) -> *mut u8 {
        self.table
    }
}

/// Table for ACPI 1.0
#[GUID("EB9D2D30-2D88-11D3-9A16-0090273FC14D", crate("crate"))]
#[derive(Debug)]
pub struct AcpiTable10 {
    table: *mut u8,
}

/// Table for SMBIOS 3
#[GUID("F2FD1544-9794-4A2C-992E-E5BBCF20E394", crate("crate"))]
#[derive(Debug)]
pub struct SMBIOS3 {
    table: *mut u8,
}

/// Table for SMBIOS
#[GUID("EB9D2D31-2D88-11D3-9A16-0090273FC14D", crate("crate"))]
#[derive(Debug)]
pub struct SMBIOS {
    table: *mut u8,
}

/// Table for SAL
#[GUID("EB9D2D32-2D88-11D3-9A16-0090273FC14D", crate("crate"))]
#[derive(Debug)]
pub struct SAL {
    table: *mut u8,
}

/// Table for MPS / MultiProcessor Specification
#[GUID("EB9D2D2F-2D88-11D3-9A16-0090273FC14D", crate("crate"))]
#[derive(Debug)]
pub struct MPS {
    table: *mut u8,
}

/// Table for ACPI 2.0 and newer
#[GUID("EB66918A-7EEF-402A-842E-931D21C38AE9", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct RuntimeProperties {
    table: *mut u8,
}

/// Table for JSON Config Data
#[GUID("87367F87-1119-41CE-AAEC-8BE0111F558A", crate("crate"))]
#[derive(Debug)]
pub struct JsonConfigData {
    table: *mut u8,
}

/// Table for JSON Capsule Data
#[GUID("35E7A725-8DD2-4CAC-8011-33CDA8109056", crate("crate"))]
#[derive(Debug)]
pub struct JsonCapsuleData {
    table: *mut u8,
}

/// Table for JSON Capsule Result
#[GUID("DBC461C3-B3DE-422A-B9B4-9886FD49A1E5", crate("crate"))]
#[derive(Debug)]
pub struct JsonCapsuleResult {
    table: *mut u8,
}

/// Flattened DTB Device Tree
#[GUID("B1B621D5-F19C-41A5-830B-D9152C69AAE0", crate("crate"))]
#[derive(Debug)]
pub struct DeviceTree {
    table: *mut u8,
}

#[GUID("DCFA911D-26EB-469F-A220-38B7DC461220", crate("crate"))]
#[derive(Debug)]
pub struct MemoryAttributes {
    table: *mut u8,
}

/// UEFI Conformance profile
#[derive(Debug)]
#[repr(C)]
pub struct RawConformanceProfile {
    ver: u16,

    /// Number of profiles
    size: u16,

    /// Array of profiles
    profiles: *const u8,
}

/// UEFI Conformance profile
///
/// If this doesn't exist, this indicates UEFI spec compliance
#[GUID("36122546-F7E7-4C8F-BD9B-EB8525B50C0B", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct ConformanceProfile {
    ver: u16,
    profiles: Vec<Guid>,
}

#[GUID("49152E77-1ADA-4764-B7A2-7AFEFED95E8B", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct DebugImageInfo {
    table: *mut u8,
}

#[GUID("D719B2CB-3D3A-4596-A3BC-DAD00E67656F", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct ImageExecInfo {
    table: *mut u8,
}

#[GUID("B122A263-3661-4F68-9929-78F8B0D62180", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct SystemResource {
    table: *mut u8,
}

#[GUID("0DE9F0EC-88B6-428F-977A-258F1D0E5E72", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct MemoryRangeCapsule {
    table: *mut u8,
}

#[GUID("6FD5B00C-D426-4283-9887-6CF5CF1CB1FE", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct UserInformation {
    table: *mut u8,
}

#[GUID("EF9FC172-A1B2-4693-B327-6D32FC416042", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct HIIDatabaseExport {
    table: *mut u8,
}

/// Deprecated Legacy EFI Properties
#[GUID("EF9FC172-A1B2-4693-B327-6D32FC416042", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct EfiProperties {
    table: *mut u8,
}

// Defined in the UEFI Platform Init spec Volume 2 Appendix B
#[GUID("05AD34BA-6F02-4214-952E-4DA0398E2BB9", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct DXEServices {
    table: *mut u8,
}

// Defined in the UEFI Platform Init spec Volume 2 Appendix B
#[GUID("7739F24C-93D7-11D4-9A3A-0090273FC14D", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct HOBlist {
    table: *mut u8,
}

// <https://github.com/tianocore/edk2/blob/f80f052277c88a67c55e107b550f504eeea947d3/MdeModulePkg/MdeModulePkg.dec#L211-L213>
#[GUID("4C19049F-4137-4DD3-9C10-8B97A83FFDFA", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct MemoryTypeInfo {
    table: *mut u8,
}

// <https://github.com/tianocore/edk2/blob/f80f052277c88a67c55e107b550f504eeea947d3/MdeModulePkg/MdeModulePkg.dec#L259-L261>
#[GUID("060CC026-4C0D-4DDA-8F41-595FEF00A502", crate("crate"))]
#[derive(Debug)]
#[repr(C)]
pub struct MemoryStatus {
    table: *mut u8,
}

impl<'tbl> ConfigTable<'tbl> for AcpiTable10 {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for AcpiTable20 {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for SMBIOS {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for SMBIOS3 {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for RuntimeProperties {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for JsonConfigData {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for JsonCapsuleData {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for JsonCapsuleResult {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for DeviceTree {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable<'tbl> for MemoryAttributes {
    type Out<'cfg> = Self where
        'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

// #[cfg(no)]
impl<'tbl> ConfigTable<'tbl> for ConformanceProfile {
    type Out<'cfg> = Self  where
    'tbl: 'cfg;

    unsafe fn from_raw(raw: *const u8) -> Self::Out<'tbl> {
        // let raw = &*raw.cast::<RawConformanceProfile>();
        // let profiles = from_raw_parts(raw.profiles.cast::<Guid>(),
        // raw.size.into()).to_vec();
        ConformanceProfile {
            ver: todo!(),
            profiles: todo!(),
            // inner: unsafe { &*raw },
            // ver: raw.ver,
            // profiles,
            // phantom: PhantomData,
        }
    }
}
