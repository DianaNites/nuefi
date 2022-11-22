use crate::{error::EfiStatus, proto::device_path::raw::RawDevicePath};

pub type AuthStateFn = unsafe extern "efiapi" fn(
    //
    this: *mut RawSecurityArch,
    status: u32,
    file: *mut RawDevicePath,
) -> EfiStatus;

pub type AuthFn = unsafe extern "efiapi" fn(
    //
    this: *mut RawSecurityArch2,
    path: *mut RawDevicePath,
    file: *mut u8,
    file_size: usize,
    boot: bool,
) -> EfiStatus;

/// Security Arch Protocol
#[repr(C)]
pub struct RawSecurityArch {
    pub auth_state: Option<AuthStateFn>,
}

impl RawSecurityArch {
    /// Create a new instance of this protocol
    pub fn create(auth_state: AuthStateFn) -> Self {
        Self {
            auth_state: Some(auth_state),
        }
    }
}

/// Security Arch2 Protocol
#[repr(C)]
pub struct RawSecurityArch2 {
    pub auth: Option<AuthFn>,
}

impl RawSecurityArch2 {
    /// Create a new instance of this protocol
    pub fn create(auth: AuthFn) -> Self {
        Self { auth: Some(auth) }
    }
}
