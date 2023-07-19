//! Function definitions for [`super::BootServices`]
//!
//! # References
//!
//! - <https://uefi.org/specs/UEFI/2.10/04_EFI_System_Table.html#efi-boot-services>
//! - <https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html>
use core::ffi::c_void;

use super::{mem::*, LocateSearch};
use crate::base::*;

// FIXME: Hack
type DevicePath = c_void;

///
pub type AllocatePages = unsafe extern "efiapi" fn(
    ty: AllocateType,
    mem_ty: MemoryType,
    pages: usize,
    memory: *mut PhysicalAddress,
) -> Status;

pub type FreePages = unsafe extern "efiapi" fn(
    //
    memory: PhysicalAddress,
    pages: usize,
) -> Status;

pub type GetMemoryMap = unsafe extern "efiapi" fn(
    map_size: *mut usize,
    map: *mut MemoryDescriptor,
    key: *mut usize,
    entry_size: *mut usize,
    entry_version: *mut u32,
) -> Status;

pub type AllocatePool = unsafe extern "efiapi" fn(
    //
    mem_ty: MemoryType,
    size: usize,
    out: *mut *mut c_void,
) -> Status;

pub type FreePool = unsafe extern "efiapi" fn(mem: *mut c_void) -> Status;

pub type InstallProtocolInterface = unsafe extern "efiapi" fn(
    handle: *mut Handle,
    guid: *mut Guid,
    interface_ty: u32,
    interface: *mut c_void,
) -> Status;

/// Locate handles, determined by the parameters
pub type LocateHandle = unsafe extern "efiapi" fn(
    search_type: LocateSearch,
    protocol: *const Guid,
    search_key: *const c_void,
    buffer_size: *mut usize,
    buffer: *mut Handle,
) -> Status;

pub type HandleProtocolFn = unsafe extern "efiapi" fn(
    handle: Handle,
    guid: *const Guid,
    interface: *mut *mut c_void,
) -> Status;

pub type LocateProtocolFn = unsafe extern "efiapi" fn(
    //
    guid: *mut Guid,
    key: *mut c_void,
    out: *mut *mut c_void,
) -> Status;

pub type InstallConfigurationTable = unsafe extern "efiapi" fn(
    //
    guid: *mut Guid,
    table: *mut c_void,
) -> Status;

pub type LoadImage = unsafe extern "efiapi" fn(
    policy: bool,
    parent: Handle,
    path: *mut DevicePath,
    source: *mut c_void,
    source_size: usize,
    out: *mut Handle,
) -> Status;

pub type StartImage = unsafe extern "efiapi" fn(
    //
    handle: Handle,
    exit_size: *mut usize,
    exit: *mut *mut c_void,
) -> Status;

pub type Exit = unsafe extern "efiapi" fn(
    handle: Handle,
    status: Status,
    data_size: usize,
    data: *mut Char16,
) -> Status;

pub type UnloadImage = unsafe extern "efiapi" fn(handle: Handle) -> Status;

pub type ExitBootServices = unsafe extern "efiapi" fn(handle: Handle, key: usize) -> Status;

pub type GetNextMonotonicCount = unsafe extern "efiapi" fn(count: *mut u64) -> Status;

pub type Stall = unsafe extern "efiapi" fn(microseconds: usize) -> Status;

pub type SetWatchdogTimer = unsafe extern "efiapi" fn(
    timeout: usize,
    code: u64,
    data_size: usize,
    data: *mut Char16,
) -> Status;

pub type OpenProtocol = unsafe extern "efiapi" fn(
    handle: Handle,
    guid: *mut Guid,
    out: *mut *mut c_void,
    agent_handle: Handle,
    controller_handle: Handle,
    attributes: u32,
) -> Status;

pub type CloseProtocol = unsafe extern "efiapi" fn(
    handle: Handle,
    guid: *mut Guid,
    agent_handle: Handle,
    controller_handle: Handle,
) -> Status;

pub type CopyMem = unsafe extern "efiapi" fn(
    //
    dest: *mut c_void,
    src: *mut c_void,
    len: usize,
);

pub type SetMem = unsafe extern "efiapi" fn(
    //
    buffer: *mut c_void,
    size: usize,
    value: u8,
);

pub type CalculateCrc32 = unsafe extern "efiapi" fn(
    //
    data: *mut c_void,
    size: usize,
    crc: *mut u32,
) -> Status;
