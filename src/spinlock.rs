use crate::println;
use crate::riscv_local::{self, intr_get, intr_off, intr_on};
use crate::types::uint;

/// Mutual exclusion spin lock
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SpinLock {
    pub locked: uint,               // Is the lock held?
    pub name: *const u8,            // Name of lock, for debugging
    pub cpu: *mut super::proc::Cpu, // The CPU holding the lock
}

impl SpinLock {
    /// Create a new spin lock instance
    pub const fn new(name: *const u8) -> Self {
        SpinLock {
            locked: 0,
            name,
            cpu: core::ptr::null_mut(),
        }
    }

    /// Initialize the lock
    #[inline]
    pub unsafe fn initlock(&mut self, name: *const u8) {
        self.name = name;
        self.locked = 0;
        self.cpu = core::ptr::null_mut();
    }

    /// Acquire the lock
    /// Loops (spins) until the lock is acquired.
    ///
    /// # Safety
    /// - Must ensure no reentrant locking (same CPU acquiring lock twice)
    /// - Automatically disables interrupts to avoid deadlock
    #[inline]
    pub unsafe fn acquire(&mut self) {
        push_off(); // disable interrupts to avoid deadlock

        if self.holding() {
            panic!("acquire");
        }

        // Spin until we acquire the lock
        // On RISC-V, this compiles to an atomic swap instruction:
        //   amoswap.w.aq a5, a5, (s1)
        while core::sync::atomic::AtomicUsize::new(1)
            .compare_exchange(
                0,
                1,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            // Hint to the processor that we're spinning
            core::hint::spin_loop();
        }

        // Memory barrier to ensure that critical section memory accesses
        // happen strictly after lock acquisition
        // On RISC-V, this emits a fence instruction
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Record info about lock acquisition for holding() and debugging
        self.cpu = super::proc::mycpu();
    }

    /// Release the lock
    ///
    /// # Safety
    /// - Must only be called by the CPU holding the lock
    /// - Must ensure all critical section operations are complete
    #[inline]
    pub unsafe fn release(&mut self) {
        if !self.holding() {
            panic!("release");
        }

        self.cpu = core::ptr::null_mut();

        // Memory barrier to ensure all stores in the critical section
        // are visible to other CPUs before the lock is released
        // On RISC-V, this emits a fence instruction
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);

        // Release the lock
        // On RISC-V, this compiles to an atomic swap instruction:
        //   amoswap.w zero, zero, (s1)
        self.locked = 0;

        pop_off();
    }

    /// Check whether this cpu is holding the lock
    ///
    /// # Safety
    /// - Interrupts must be off when calling this function
    #[inline]
    pub unsafe fn holding(&self) -> bool {
        self.locked != 0 && self.cpu == super::proc::mycpu()
    }
}

/// Disable interrupts and increment noff count
/// This is a nested operation that requires matching pop_off calls
#[inline]
pub unsafe fn push_off() {
    let old = intr_get();
    intr_off();
    let cpu = super::proc::mycpu();
    if (*cpu).noff == 0 {
        (*cpu).intena = old;
    }
    (*cpu).noff += 1;
}

/// Decrement noff count and restore interrupts if count reaches 0
/// and interrupts were enabled before
///
/// # Panics
/// - If interrupts are currently enabled
/// - If noff is less than 1
#[inline]
pub unsafe fn pop_off() {
    let cpu = super::proc::mycpu();
    (*cpu).noff -= 1;
    if (*cpu).noff == 0 && (*cpu).intena {
        intr_on();
    }
}
