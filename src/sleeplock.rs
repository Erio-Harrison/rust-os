use crate::{
    proc::{myproc, sleep, wakeup},
    spinlock::SpinLock,
    types::uint,
};

/// Long-term locks for processes
#[derive(Clone, Copy)]
pub struct Sleeplock {
    pub locked: uint,    // Is the lock held?
    pub lk: SpinLock,    // spinlock protecting this sleep lock
    pub name: *const u8, // Name of lock, for debugging
    pub pid: uint,       // Process holding lock
}

impl Sleeplock {
    /// Create a new sleep lock instance
    pub const fn new(name: *const u8) -> Self {
        Sleeplock {
            locked: 0,
            lk: SpinLock::new("sleep lock\0".as_ptr()),
            name,
            pid: 0,
        }
    }

    /// Initialize a sleep lock
    #[inline]
    pub unsafe fn init(&mut self, name: *const u8) {
        self.lk.initlock("sleep lock\0".as_ptr());
        self.name = name;
        self.locked = 0;
        self.pid = 0;
    }

    /// Acquire the sleep lock
    #[inline]
    pub unsafe fn acquire(&mut self) {
        self.lk.acquire();
        while self.locked != 0 {
            sleep(self as *mut Sleeplock as *mut u8, &mut self.lk);
        }
        self.locked = 1;
        self.pid = (*myproc()).pid as u32;
        self.lk.release();
    }

    /// Release the sleep lock
    #[inline]
    pub unsafe fn release(&mut self) {
        self.lk.acquire();
        self.locked = 0;
        self.pid = 0;
        wakeup(self as *mut Sleeplock as *mut u8);
        self.lk.release();
    }

    /// Check if the current process holds the lock
    #[inline]
    pub unsafe fn holding(&mut self) -> bool {
        self.lk.acquire();
        let r = self.locked != 0 && (self.pid == (*myproc()).pid.try_into().unwrap());
        self.lk.release();
        r
    }
}
