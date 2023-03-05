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
    impl<'tbl> Sealed for ConformanceProfile<'tbl> {}
}

pub trait ConfigTable: Entity + imp::Sealed {
    type Out<'tbl>
    where
        Self: 'tbl;

    /// # Safety
    ///
    /// - `raw` must be valid for this table
    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl>;
}

#[derive(Debug)]
#[repr(transparent)]
pub struct GenericConfig<'tbl> {
    config: RawConfigurationTable,
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

    /// Name of this protocol, if known
    pub fn name(&self) -> Option<&'static str> {
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
        } else {
            None
        }
    }

    /// If this generic table is `T`, then return
    // This lives as long as `'tbl`, which can only come from
    // the `SystemTable`.
    pub fn as_table<T: ConfigTable>(&self) -> Option<T::Out<'tbl>> {
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
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RuntimeProperties {
    version: u16,
    len: u16,
    supported: u32,
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
pub struct ConformanceProfile<'tbl> {
    ver: u16,
    profiles: Vec<Guid>,

    /// Lifetime of the SystemTable
    phantom: PhantomData<&'tbl ()>,
}

impl ConfigTable for AcpiTable10 {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for AcpiTable20 {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for SMBIOS {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for SMBIOS3 {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for RuntimeProperties {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        *raw.cast::<RuntimeProperties>()
    }
}

impl ConfigTable for JsonConfigData {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for JsonCapsuleData {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for JsonCapsuleResult {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for DeviceTree {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl ConfigTable for MemoryAttributes {
    type Out<'tbl> = Self;

    unsafe fn from_raw<'tbl>(raw: *const u8) -> Self::Out<'tbl> {
        Self {
            table: raw.cast_mut(),
        }
    }
}

impl<'tbl> ConfigTable for ConformanceProfile<'tbl> {
    type Out<'tbl2> = ConformanceProfile<'tbl2> where Self: 'tbl2;

    unsafe fn from_raw<'tbl3>(raw: *const u8) -> Self::Out<'tbl3> {
        let raw = &*raw.cast::<RawConformanceProfile>();
        let profiles = from_raw_parts(raw.profiles.cast::<Guid>(), raw.size).to_vec();
        ConformanceProfile {
            ver: raw.ver,
            profiles,
            phantom: PhantomData,
        }
    }
}
