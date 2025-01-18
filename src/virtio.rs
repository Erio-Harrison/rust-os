//! Virtio device definitions.
//! For both the mmio interface, and virtio descriptors.
//! Only tested with qemu.
//!
//! The virtio spec:
//! https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.pdf

use crate::{
    fs::BSIZE,
    kalloc::kalloc,
    memlayout::VIRTIO0,
    proc::{sleep, wakeup},
    riscv::PGSIZE,
    sleeplock::Sleeplock,
    spinlock::SpinLock,
    string::memset,
    types::uint,
};
use core::sync::atomic::Ordering;
use core::{ptr, sync::atomic::fence};

// Virtio mmio control registers, mapped starting at 0x10001000.
pub const VIRTIO_MMIO_MAGIC_VALUE: u64 = 0x000; // 0x74726976
pub const VIRTIO_MMIO_VERSION: u64 = 0x004; // version; should be 2
pub const VIRTIO_MMIO_DEVICE_ID: u64 = 0x008; // device type; 1 is net, 2 is disk
pub const VIRTIO_MMIO_VENDOR_ID: u64 = 0x00c; // 0x554d4551
pub const VIRTIO_MMIO_DEVICE_FEATURES: u64 = 0x010;
pub const VIRTIO_MMIO_DRIVER_FEATURES: u64 = 0x020;
pub const VIRTIO_MMIO_QUEUE_SEL: u64 = 0x030; // select queue, write-only
pub const VIRTIO_MMIO_QUEUE_NUM_MAX: u64 = 0x034; // max size of current queue, read-only
pub const VIRTIO_MMIO_QUEUE_NUM: u64 = 0x038; // size of current queue, write-only
pub const VIRTIO_MMIO_QUEUE_READY: u64 = 0x044; // ready bit
pub const VIRTIO_MMIO_QUEUE_NOTIFY: u64 = 0x050; // write-only
pub const VIRTIO_MMIO_INTERRUPT_STATUS: u64 = 0x060; // read-only
pub const VIRTIO_MMIO_INTERRUPT_ACK: u64 = 0x064; // write-only
pub const VIRTIO_MMIO_STATUS: u64 = 0x070; // read/write
pub const VIRTIO_MMIO_QUEUE_DESC_LOW: u64 = 0x080; // physical address for descriptor table, write-only
pub const VIRTIO_MMIO_QUEUE_DESC_HIGH: u64 = 0x084;
pub const VIRTIO_MMIO_DRIVER_DESC_LOW: u64 = 0x090; // physical address for available ring, write-only
pub const VIRTIO_MMIO_DRIVER_DESC_HIGH: u64 = 0x094;
pub const VIRTIO_MMIO_DEVICE_DESC_LOW: u64 = 0x0a0; // physical address for used ring, write-only
pub const VIRTIO_MMIO_DEVICE_DESC_HIGH: u64 = 0x0a4;

// Status register bits
pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32 = 1;
pub const VIRTIO_CONFIG_S_DRIVER: u32 = 2;
pub const VIRTIO_CONFIG_S_DRIVER_OK: u32 = 4;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u32 = 8;

// Device feature bits
pub const VIRTIO_BLK_F_RO: u32 = 5; // Disk is read-only
pub const VIRTIO_BLK_F_SCSI: u32 = 7; // Supports scsi command passthru
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11; // Writeback mode available in config
pub const VIRTIO_BLK_F_MQ: u32 = 12; // support more than one vq
pub const VIRTIO_F_ANY_LAYOUT: u32 = 27;
pub const VIRTIO_RING_F_INDIRECT_DESC: u32 = 28;
pub const VIRTIO_RING_F_EVENT_IDX: u32 = 29;

// This many virtio descriptors.
// Must be a power of two.
pub const NUM: usize = 8;

// Descriptor flags
pub const VRING_DESC_F_NEXT: u16 = 1; // chained with another descriptor
pub const VRING_DESC_F_WRITE: u16 = 2; // device writes (vs read)

/// A single descriptor, from the spec.
#[repr(C)]
pub struct VirtqDesc {
    pub addr: u64,  // Buffer Address
    pub len: u32,   // Buffer Length
    pub flags: u16, // The flags as indicated above
    pub next: u16,  // Next field if flags & NEXT
}

/// The (entire) avail ring, from the spec.
#[repr(C)]
pub struct VirtqAvail {
    pub flags: u16,       // Always zero
    pub idx: u16,         // Driver will write ring[idx] next
    pub ring: [u16; NUM], // Descriptor numbers of chain heads
    pub unused: u16,
}

/// One entry in the "used" ring, with which the
/// device tells the driver about completed requests.
#[repr(C)]
pub struct VirtqUsedElem {
    pub id: u32,  // Index of start of completed descriptor chain
    pub len: u32, // Length of completed request
}

#[repr(C)]
pub struct VirtqUsed {
    pub flags: u16, // Always zero
    pub idx: u16,   // Device increments when it adds a ring[] entry
    pub ring: [VirtqUsedElem; NUM],
}

// These are specific to virtio block devices, e.g. disks,
// described in Section 5.2 of the spec.
pub const VIRTIO_BLK_T_IN: u32 = 0; // read the disk
pub const VIRTIO_BLK_T_OUT: u32 = 1; // write the disk

/// The format of the first descriptor in a disk request.
/// To be followed by two more descriptors containing
/// the block, and a one-byte status.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VirtioBlkReq {
    pub type_: u32, // VIRTIO_BLK_T_IN or ..._OUT
    pub reserved: u32,
    pub sector: u64,
}

/// Buffer structure
#[repr(C)]
pub struct Buffer {
    pub valid: i32,        // Has data been read from disk?
    pub disk: i32,         // Does disk "own" buf?
    pub dev: uint,         // Device number
    pub blockno: uint,     // Block number
    pub lock: Sleeplock,   // Sleep lock
    pub refcnt: uint,      // Reference count
    pub prev: *mut Buffer, // LRU cache list
    pub next: *mut Buffer, // LRU cache list
    pub data: [u8; BSIZE], // Data
}

#[repr(C)]
#[derive(Copy, Clone)]
struct DiskInfo {
    b: *mut Buffer,
    status: u8,
}

/// Disk device structure
#[repr(C)]
pub struct Disk {
    desc: *mut VirtqDesc,   // Descriptor array
    avail: *mut VirtqAvail, // Available ring
    used: *mut VirtqUsed,   // Used ring

    free: [u8; NUM], // Is a descriptor free?
    used_idx: u16,   // We've looked this far in used[2..NUM]

    info: [DiskInfo; NUM],    // Track info about in-flight operations
    ops: [VirtioBlkReq; NUM], // Disk command headers

    vdisk_lock: SpinLock, // Protect disk data structure
}

// Global disk instance
pub(crate) static mut DISK: Disk = Disk {
    desc: ptr::null_mut(),
    avail: ptr::null_mut(),
    used: ptr::null_mut(),
    free: [0; NUM],
    used_idx: 0,
    info: [DiskInfo {
        b: ptr::null_mut(),
        status: 0,
    }; NUM],
    ops: [VirtioBlkReq {
        type_: 0,
        reserved: 0,
        sector: 0,
    }; NUM],
    vdisk_lock: SpinLock::new("virtio_disk\0".as_bytes().as_ptr()),
};

/// Read from MMIO register
unsafe fn read_reg(offset: u64) -> u32 {
    ptr::read_volatile((VIRTIO0 as *const u32).add(offset as usize))
}

/// Write to MMIO register
unsafe fn write_reg(offset: u64, value: u32) {
    ptr::write_volatile((VIRTIO0 as *mut u32).add(offset as usize), value);
}

/// Initialize virtio disk device
pub unsafe fn virtio_disk_init() {
    let mut status: u32 = 0;

    // Initialize disk lock
    DISK.vdisk_lock
        .initlock("virtio_disk\0".as_bytes().as_ptr());

    // Check device identification
    if read_reg(VIRTIO_MMIO_MAGIC_VALUE) != 0x74726976
        || read_reg(VIRTIO_MMIO_VERSION) != 2
        || read_reg(VIRTIO_MMIO_DEVICE_ID) != 2
        || read_reg(VIRTIO_MMIO_VENDOR_ID) != 0x554d4551
    {
        panic!("could not find virtio disk");
    }

    // Reset device
    write_reg(VIRTIO_MMIO_STATUS, status);

    // Set ACKNOWLEDGE status bit
    status |= VIRTIO_CONFIG_S_ACKNOWLEDGE;
    write_reg(VIRTIO_MMIO_STATUS, status);

    // Set DRIVER status bit
    status |= VIRTIO_CONFIG_S_DRIVER;
    write_reg(VIRTIO_MMIO_STATUS, status);

    // Negotiate features
    let mut features = read_reg(VIRTIO_MMIO_DEVICE_FEATURES);
    features &= !(1 << VIRTIO_BLK_F_RO);
    features &= !(1 << VIRTIO_BLK_F_SCSI);
    features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
    features &= !(1 << VIRTIO_BLK_F_MQ);
    features &= !(1 << VIRTIO_F_ANY_LAYOUT);
    features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
    features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);
    write_reg(VIRTIO_MMIO_DRIVER_FEATURES, features);

    // Tell device that feature negotiation is complete
    status |= VIRTIO_CONFIG_S_FEATURES_OK;
    write_reg(VIRTIO_MMIO_STATUS, status);

    // Re-read status to ensure FEATURES_OK is set
    status = read_reg(VIRTIO_MMIO_STATUS);
    if (status & VIRTIO_CONFIG_S_FEATURES_OK) == 0 {
        panic!("virtio disk FEATURES_OK unset");
    }

    // Initialize queue 0
    write_reg(VIRTIO_MMIO_QUEUE_SEL, 0);

    // Ensure queue 0 is not in use
    if read_reg(VIRTIO_MMIO_QUEUE_READY) != 0 {
        panic!("virtio disk should not be ready");
    }

    // Check maximum queue size
    let max = read_reg(VIRTIO_MMIO_QUEUE_NUM_MAX);
    if max == 0 {
        panic!("virtio disk has no queue 0");
    }
    if max < NUM as u32 {
        panic!("virtio disk max queue too short");
    }

    // Allocate and zero queue memory
    DISK.desc = kalloc() as *mut VirtqDesc;
    DISK.avail = kalloc() as *mut VirtqAvail;
    DISK.used = kalloc() as *mut VirtqUsed;

    if DISK.desc.is_null() || DISK.avail.is_null() || DISK.used.is_null() {
        panic!("virtio disk kalloc");
    }

    memset(DISK.desc as *mut u8, 0, PGSIZE.try_into().unwrap());
    memset(DISK.avail as *mut u8, 0, PGSIZE.try_into().unwrap());
    memset(DISK.used as *mut u8, 0, PGSIZE.try_into().unwrap());

    // Set queue size
    write_reg(VIRTIO_MMIO_QUEUE_NUM, NUM as u32);

    // Write physical addresses
    write_reg(VIRTIO_MMIO_QUEUE_DESC_LOW, DISK.desc as u32);
    write_reg(
        VIRTIO_MMIO_QUEUE_DESC_HIGH,
        ((DISK.desc as u64) >> 32) as u32,
    );
    write_reg(VIRTIO_MMIO_DRIVER_DESC_LOW, DISK.avail as u32);
    write_reg(
        VIRTIO_MMIO_DRIVER_DESC_HIGH,
        ((DISK.avail as u64) >> 32) as u32,
    );
    write_reg(VIRTIO_MMIO_DEVICE_DESC_LOW, DISK.used as u32);
    write_reg(
        VIRTIO_MMIO_DEVICE_DESC_HIGH,
        ((DISK.used as u64) >> 32) as u32,
    );

    // Mark queue as ready
    write_reg(VIRTIO_MMIO_QUEUE_READY, 1);

    // Initialize all descriptors as free
    for i in 0..NUM {
        DISK.free[i] = 1;
    }

    // Tell device we're completely ready
    status |= VIRTIO_CONFIG_S_DRIVER_OK;
    write_reg(VIRTIO_MMIO_STATUS, status);
}

impl Disk {
    /// Find a free descriptor, mark it non-free, return its index
    unsafe fn alloc_desc(&mut self) -> i32 {
        for i in 0..NUM {
            if self.free[i] == 1 {
                self.free[i] = 0;
                return i as i32;
            }
        }
        -1
    }

    /// Mark a descriptor as free
    unsafe fn free_desc(&mut self, i: i32) {
        if i as usize >= NUM {
            panic!("free_desc 1");
        }
        if self.free[i as usize] != 0 {
            panic!("free_desc 2");
        }

        // Clear the descriptor
        (*self.desc.add(i as usize)).addr = 0;
        (*self.desc.add(i as usize)).len = 0;
        (*self.desc.add(i as usize)).flags = 0;
        (*self.desc.add(i as usize)).next = 0;

        // Mark as free
        self.free[i as usize] = 1;

        // Wake up processes waiting for descriptors
        wakeup(self.free.as_mut_ptr());
    }

    /// Free a chain of descriptors
    unsafe fn free_chain(&mut self, mut i: i32) {
        loop {
            let flag = (*self.desc.add(i as usize)).flags;
            let nxt = (*self.desc.add(i as usize)).next;
            self.free_desc(i);

            if (flag & VRING_DESC_F_NEXT) != 0 {
                i = nxt as i32;
            } else {
                break;
            }
        }
    }

    /// Allocate three descriptors (they need not be contiguous).
    /// Disk transfers always use three descriptors.
    unsafe fn alloc3_desc(&mut self, idx: &mut [i32; 3]) -> i32 {
        for i in 0..3 {
            idx[i] = self.alloc_desc();
            if idx[i] < 0 {
                // If allocation failed, free any descriptors we managed to allocate
                for j in 0..i {
                    self.free_desc(idx[j]);
                }
                return -1;
            }
        }
        0
    }

    /// Read/write from the virtio disk
    pub unsafe fn virtio_disk_rw(&mut self, b: *mut Buffer, write: bool) {
        let sector = (*b).blockno * (BSIZE / 512) as u32;

        self.vdisk_lock.acquire();

        // The spec's Section 5.2 says that legacy block operations use
        // three descriptors: one for type/reserved/sector, one for the
        // data, one for a 1-byte status result.

        // Allocate the three descriptors
        let mut idx = [0i32; 3];
        loop {
            if self.alloc3_desc(&mut idx) == 0 {
                break;
            }
            sleep(self.free.as_mut_ptr(), &mut self.vdisk_lock);
        }

        // Format the three descriptors.
        // qemu's virtio-blk.c reads them.
        let buf0 = &mut self.ops[idx[0] as usize];

        buf0.type_ = if write {
            VIRTIO_BLK_T_OUT
        } else {
            VIRTIO_BLK_T_IN
        };
        buf0.reserved = 0;
        buf0.sector = sector as u64;

        // Set up descriptor chain
        (*self.desc.add(idx[0] as usize)).addr = buf0 as *mut VirtioBlkReq as u64;
        (*self.desc.add(idx[0] as usize)).len = core::mem::size_of::<VirtioBlkReq>() as u32;
        (*self.desc.add(idx[0] as usize)).flags = VRING_DESC_F_NEXT;
        (*self.desc.add(idx[0] as usize)).next = idx[1] as u16;

        (*self.desc.add(idx[1] as usize)).addr = (*b).data.as_ptr() as u64;
        (*self.desc.add(idx[1] as usize)).len = BSIZE as u32;
        (*self.desc.add(idx[1] as usize)).flags = if write {
            VRING_DESC_F_NEXT // device reads b->data
        } else {
            VRING_DESC_F_NEXT | VRING_DESC_F_WRITE // device writes b->data
        };
        (*self.desc.add(idx[1] as usize)).next = idx[2] as u16;

        self.info[idx[0] as usize].status = 0xff; // device writes 0 on success
        (*self.desc.add(idx[2] as usize)).addr =
            &mut self.info[idx[0] as usize].status as *mut u8 as u64;
        (*self.desc.add(idx[2] as usize)).len = 1;
        (*self.desc.add(idx[2] as usize)).flags = VRING_DESC_F_WRITE; // device writes the status
        (*self.desc.add(idx[2] as usize)).next = 0;

        // Record struct buf for virtio_disk_intr()
        (*b).disk = 1;
        self.info[idx[0] as usize].b = b;

        // Tell the device the first index in our chain of descriptors
        (*self.avail).ring[(*self.avail).idx as usize % NUM] = idx[0] as u16;

        fence(Ordering::SeqCst);

        // Tell the device another avail ring entry is available
        (*self.avail).idx = (*self.avail).idx.wrapping_add(1); // not % NUM

        fence(Ordering::SeqCst);

        // Value is queue number
        write_reg(VIRTIO_MMIO_QUEUE_NOTIFY, 0);

        // Wait for virtio_disk_intr() to say request has finished
        while (*b).disk == 1 {
            sleep(b as *mut u8, &mut self.vdisk_lock);
        }

        self.info[idx[0] as usize].b = core::ptr::null_mut();
        self.free_chain(idx[0]);

        self.vdisk_lock.release();
    }

    /// Handle a disk interrupt
    pub unsafe fn virtio_disk_intr(&mut self) {
        self.vdisk_lock.acquire();

        // The device won't raise another interrupt until we tell it
        // we've seen this interrupt, which the following line does.
        // This may race with the device writing new entries to
        // the "used" ring, in which case we may process the new
        // completion entries in this interrupt, and have nothing to do
        // in the next interrupt, which is harmless.
        write_reg(
            VIRTIO_MMIO_INTERRUPT_ACK,
            read_reg(VIRTIO_MMIO_INTERRUPT_STATUS) & 0x3,
        );

        fence(Ordering::SeqCst);

        // The device increments disk.used->idx when it
        // adds an entry to the used ring.
        while self.used_idx != (*self.used).idx {
            fence(Ordering::SeqCst);
            let id = (*self.used).ring[self.used_idx as usize % NUM].id;

            if self.info[id as usize].status != 0 {
                panic!("virtio_disk_intr status");
            }

            let b = self.info[id as usize].b;
            (*b).disk = 0; // disk is done with buf
            wakeup(b as *mut u8);

            self.used_idx = self.used_idx.wrapping_add(1);
        }

        self.vdisk_lock.release();
    }
}
