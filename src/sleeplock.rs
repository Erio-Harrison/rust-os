use crate::{spinlock::SpinLock, types::uint};

// Long-term locks for processes
pub struct Sleeplock {
    pub locked: uint,        // Is the lock held?
    pub lk: SpinLock,      // spinlock protecting this sleep lock
    
    // For debugging:
    pub name: *const u8,    // Name of lock
    pub pid: uint,          // Process holding lock
 }