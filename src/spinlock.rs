use crate::types::uint;

#[repr(C)]
pub struct SpinLock {
    pub locked: uint,      
    pub name: *const u8,      
    pub cpu: *mut super::proc::Cpu, 
}

impl SpinLock {
    pub const fn new(name: *const u8) -> Self {
        SpinLock {
            locked: 0,
            name,
            cpu: core::ptr::null_mut(),
        }
    }
}