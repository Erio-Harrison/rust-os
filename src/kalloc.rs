use core::ptr::{self};
use crate::riscv::*;
use crate::memlayout::*;
use crate::riscv::pgroundup;
use super::spinlock::SpinLock;

/// Free memory node - forms a linked list of free pages
#[repr(C)]
struct Run {
    next: *mut Run,
}

/// Kernel memory allocator state
struct KMem {
    lock: SpinLock,
    freelist: *mut Run,
}

// Global allocator instance
static mut KMEM: KMem = KMem {
    lock: SpinLock::new(b"kmem\0" as *const u8),
    freelist: ptr::null_mut(),
};

extern "C" {
    // First address after kernel, defined by kernel.ld
    static end: [u8; 0];
}

/// Initialize the allocator
/// 
/// # Safety
/// This function must only be called once during kernel initialization
pub unsafe fn kinit() {
    KMEM.lock.initlock(b"kmem\0" as *const u8);
    freerange(end.as_ptr(), PHYSTOP as *mut u8);
}

/// Free a range of physical memory
/// 
/// # Safety
/// - pa_start and pa_end must be valid physical addresses
/// - The range must not overlap with already allocated memory
unsafe fn freerange(pa_start: *const u8, pa_end: *mut u8) {
    let mut p = pgroundup(pa_start as u64) as *mut u8;
    while (p as u64) + PGSIZE <= pa_end as u64 {
        kfree(p);
        p = p.add(PGSIZE.try_into().unwrap());
    }
}

/// Free the page of physical memory pointed at by pa,
/// which normally should have been returned by a call to kalloc().
/// 
/// # Safety
/// - pa must point to a page previously allocated by kalloc()
/// - The page must not be freed more than once
pub unsafe fn kfree(pa: *mut u8) {
    if (pa as u64) % PGSIZE != 0 || 
       (pa as *const u8) < end.as_ptr() || 
       pa as u64 >= PHYSTOP {
        panic!("kfree");
    }

    // Fill with junk to catch dangling refs
    ptr::write_bytes(pa, 1, PGSIZE.try_into().unwrap());

    let r = pa as *mut Run;
    KMEM.lock.acquire();
    (*r).next = KMEM.freelist;
    KMEM.freelist = r;
    KMEM.lock.release();
}

/// Allocate one 4096-byte page of physical memory.
/// Returns a pointer that the kernel can use.
/// Returns null if the memory cannot be allocated.
/// 
/// # Safety
/// The returned memory is not initialized
pub unsafe fn kalloc() -> *mut u8 {
    KMEM.lock.acquire();
    
    let r = KMEM.freelist;
    if !r.is_null() {
        KMEM.freelist = (*r).next;
    }
    
    KMEM.lock.release();

    if !r.is_null() {
        // Fill with junk
        ptr::write_bytes(r as *mut u8, 5, PGSIZE.try_into().unwrap());
    }
    
    r as *mut u8
}