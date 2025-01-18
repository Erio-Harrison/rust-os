//! Buffer cache.
//!
//! The buffer cache is a linked list of buf structures holding
//! cached copies of disk block contents. Caching disk blocks
//! in memory reduces the number of disk reads and also provides
//! a synchronization point for disk blocks used by multiple processes.
//!
//! Interface:
//! * To get a buffer for a particular disk block, call bread.
//! * After changing buffer data, call bwrite to write it to disk.
//! * When done with the buffer, call brelse.
//! * Do not use the buffer after calling brelse.
//! * Only one process at a time can use a buffer,
//!   so do not keep them longer than necessary.

use crate::fs::BSIZE;
use crate::virtio::{Buffer, DISK};
use crate::{kalloc::kalloc, sleeplock::Sleeplock, spinlock::SpinLock};
use core::ptr;

const NBUF: usize = 30;

struct Bcache {
    lock: SpinLock,    // Protects the buffer cache
    bufs: *mut Buffer, // The buffer cache array
    nbufs: usize,      // Number of buffers
    head: Buffer,      // Dummy head of linked list of buffers
}

static mut BCACHE: Bcache = Bcache {
    lock: SpinLock::new("bcache\0".as_ptr()),
    bufs: ptr::null_mut(),
    nbufs: NBUF,
    head: Buffer {
        valid: 0,
        disk: 0,
        dev: 0,
        blockno: 0,
        lock: Sleeplock {
            locked: 0,
            lk: SpinLock::new("bcache_head\0".as_ptr()),
            name: "bcache_head\0".as_ptr(),
            pid: 0,
        },
        refcnt: 0,
        prev: ptr::null_mut(),
        next: ptr::null_mut(),
        data: [0; BSIZE],
    },
};

/// Initialize buffer cache
pub unsafe fn binit() {
    // Allocate memory for buffer array
    let size = core::mem::size_of::<Buffer>() * NBUF;
    let buffers = kalloc() as *mut Buffer;
    if buffers.is_null() {
        panic!("binit: buffer allocation failed");
    }
    BCACHE.bufs = buffers;

    // Initialize each buffer
    for i in 0..NBUF {
        let buf = &mut *buffers.add(i);
        buf.valid = 0;
        buf.disk = 0;
        buf.dev = 0;
        buf.blockno = 0;
        buf.lock = Sleeplock {
            locked: 0,
            lk: SpinLock::new("buffer\0".as_ptr()),
            name: "buffer\0".as_ptr(),
            pid: 0,
        };
        buf.refcnt = 0;
        buf.prev = ptr::null_mut();
        buf.next = ptr::null_mut();
        buf.data = [0; BSIZE];
    }

    // Create linked list of buffers
    let head_ptr = &mut BCACHE.head as *mut Buffer;
    BCACHE.head.prev = head_ptr;
    BCACHE.head.next = head_ptr;

    for i in 0..NBUF {
        let b = buffers.add(i);
        (*b).next = BCACHE.head.next;
        (*b).prev = head_ptr;
        (*BCACHE.head.next).prev = b;
        BCACHE.head.next = b;
    }
}

/// Look through buffer cache for block on device dev.
/// If not found, allocate a buffer.
/// In either case, return locked buffer.
unsafe fn bget(dev: u32, blockno: u32) -> *mut Buffer {
    BCACHE.lock.acquire();

    // Is the block already cached?
    let mut b = BCACHE.head.next;
    while b != &mut BCACHE.head as *mut Buffer {
        if (*b).dev == dev && (*b).blockno == blockno {
            (*b).refcnt += 1;
            BCACHE.lock.release();
            (*b).lock.lk.acquire();
            return b;
        }
        b = (*b).next;
    }

    // Not cached.
    // Recycle the least recently used (LRU) unused buffer.
    b = BCACHE.head.prev;
    while b != &mut BCACHE.head as *mut Buffer {
        if (*b).refcnt == 0 {
            (*b).dev = dev;
            (*b).blockno = blockno;
            (*b).valid = 0;
            (*b).refcnt = 1;
            BCACHE.lock.release();
            (*b).lock.lk.acquire();
            return b;
        }
        b = (*b).prev;
    }
    panic!("bget: no buffers");
}

/// Return a locked buf with the contents of the indicated block.
pub unsafe fn bread(dev: u32, blockno: u32) -> *mut Buffer {
    let b = bget(dev, blockno);
    if (*b).valid == 0 {
        // Read from disk using global DISK instance
        DISK.virtio_disk_rw(b, false);
        (*b).valid = 1;
    }
    b
}

/// Write b's contents to disk. Must be locked.
pub unsafe fn bwrite(b: *mut Buffer) {
    if !(*b).lock.holding() {
        panic!("bwrite");
    }
    // Write to disk using global DISK instance
    DISK.virtio_disk_rw(b, true);
}

/// Release a locked buffer.
/// Move to the head of the most-recently-used list.
pub unsafe fn brelse(b: *mut Buffer) {
    if !(*b).lock.lk.holding() {
        panic!("brelse");
    }

    (*b).lock.lk.release();

    BCACHE.lock.acquire();
    (*b).refcnt -= 1;
    if (*b).refcnt == 0 {
        // no one is waiting for it.
        // remove from current position
        (*(*b).next).prev = (*b).prev;
        (*(*b).prev).next = (*b).next;
        // move to front of LRU list
        (*b).next = BCACHE.head.next;
        (*b).prev = &mut BCACHE.head as *mut Buffer;
        (*BCACHE.head.next).prev = b;
        BCACHE.head.next = b;
    }

    BCACHE.lock.release();
}

/// Increment refcnt for a buffer
pub unsafe fn bpin(b: *mut Buffer) {
    BCACHE.lock.acquire();
    (*b).refcnt += 1;
    BCACHE.lock.release();
}

/// Decrement refcnt for a buffer
pub unsafe fn bunpin(b: *mut Buffer) {
    BCACHE.lock.acquire();
    (*b).refcnt -= 1;
    BCACHE.lock.release();
}
