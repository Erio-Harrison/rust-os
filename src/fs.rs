//! On-disk file system format.
//! Both the kernel and user programs use these type definitions.

use core::{cmp::min, mem::size_of, ptr};

use crate::{
    bio::{bread, brelse},
    file::Inode,
    log::Log,
    param::NINODE,
    println,
    proc::{either_copyin, either_copyout},
    sleeplock::Sleeplock,
    spinlock::SpinLock,
    stat::Stat,
};

/// Block size in bytes
pub const BSIZE: usize = 1024;

/// Root inode number
pub const ROOTINO: u32 = 1;

/// Magic number identifying the file system
pub const FSMAGIC: u32 = 0x10203040;

/// Number of direct block addresses in inode
pub const NDIRECT: usize = 12;

/// Number of indirect block addresses (BSIZE / size of uint)
pub const NINDIRECT: usize = BSIZE / size_of::<u32>();

/// Maximum file size in blocks (direct + indirect)
pub const MAXFILE: usize = NDIRECT + NINDIRECT;

/// Maximum length of directory entry name
pub const DIRSIZ: usize = 14;

/// Number of inodes per block
pub const IPB: usize = BSIZE / size_of::<DiskInode>();

/// Number of bitmap bits per block
pub const BPB: usize = BSIZE * 8;

/// File types for disk inodes
pub const T_DIR: i16 = 1; // Directory
pub const T_FILE: i16 = 2; // File
pub const T_DEVICE: i16 = 3; // Device

/// Calculate block number containing inode i
#[macro_export]
macro_rules! IBLOCK {
    ($i:expr, $sb:expr) => {
        (($i) / IPB as u32 + ($sb).inodestart)
    };
}

/// Calculate block of free map containing bit for block b
#[macro_export]
macro_rules! BBLOCK {
    ($b:expr, $sb:expr) => {
        (($b) / BPB as u32 + ($sb).bmapstart)
    };
}

/// Disk layout:
/// [ boot block | super block | log | inode blocks |
///                                   free bit map | data blocks]
///
/// The super block describes the disk layout.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Superblock {
    pub magic: u32,      // Must be FSMAGIC
    pub size: u32,       // Size of file system image (blocks)
    pub nblocks: u32,    // Number of data blocks
    pub ninodes: u32,    // Number of inodes
    pub nlog: u32,       // Number of log blocks
    pub logstart: u32,   // Block number of first log block
    pub inodestart: u32, // Block number of first inode block
    pub bmapstart: u32,  // Block number of first free map block
}

impl Superblock {
    /// Create a new empty superblock
    pub const fn new() -> Self {
        Self {
            magic: 0,
            size: 0,
            nblocks: 0,
            ninodes: 0,
            nlog: 0,
            logstart: 0,
            inodestart: 0,
            bmapstart: 0,
        }
    }
}

/// On-disk inode structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DiskInode {
    pub type_: i16,                // File type
    pub major: i16,                // Major device number (T_DEVICE only)
    pub minor: i16,                // Minor device number (T_DEVICE only)
    pub nlink: i16,                // Number of links to inode in file system
    pub size: u32,                 // Size of file (bytes)
    pub addrs: [u32; NDIRECT + 1], // Data block addresses + 1 indirect
}

/// Directory entry structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dirent {
    pub inum: u16,          // Inode number
    pub name: [u8; DIRSIZ], // File name
}

/// File type constants
pub mod filetype {
    pub const T_DIR: i16 = 1; // Directory
    pub const T_FILE: i16 = 2; // File
    pub const T_DEVICE: i16 = 3; // Device
}

// There should be one superblock per disk device, but we run with
// only one device
static mut SB: Superblock = Superblock {
    magic: 0,
    size: 0,
    nblocks: 0,
    ninodes: 0,
    nlog: 0,
    logstart: 0,
    inodestart: 0,
    bmapstart: 0,
};

/// Read the super block.
unsafe fn readsb(dev: u32, sb: &mut Superblock) {
    let bp = bread(dev, 1);
    ptr::copy_nonoverlapping(
        (*bp).data.as_ptr(),
        sb as *mut Superblock as *mut u8,
        core::mem::size_of::<Superblock>(),
    );
    brelse(bp);
}

/// File system state
pub struct Filesystem {
    pub dev: u32,               // Device number
    pub superblock: Superblock, // Superblock (only one device in xv6)
    pub log: Log,               // Transaction log
}

// Global filesystem instance
pub static mut FS: Filesystem = Filesystem {
    dev: 0,
    superblock: Superblock {
        magic: 0,
        size: 0,
        nblocks: 0,
        ninodes: 0,
        nlog: 0,
        logstart: 0,
        inodestart: 0,
        bmapstart: 0,
    },
    log: Log::new(),
};

impl Filesystem {
    /// Initialize the filesystem
    pub unsafe fn init(&mut self, dev: u32) {
        self.dev = dev;
        self.read_superblock();

        if self.superblock.magic != FSMAGIC {
            panic!("invalid file system");
        }

        self.log.init(dev as i32, &self.superblock);
    }

    /// Read superblock from disk
    unsafe fn read_superblock(&mut self) {
        let bp = bread(self.dev, 1);
        self.superblock = *((*bp).data.as_ptr() as *const Superblock);
        brelse(bp);
    }

    /// Zero a block
    unsafe fn bzero(&mut self, bno: u32) {
        let bp = bread(self.dev, bno);
        ptr::write_bytes((*bp).data.as_mut_ptr(), 0, BSIZE);
        self.log.write(bp);
        brelse(bp);
    }

    /// Allocate a zeroed disk block
    pub unsafe fn balloc(&mut self) -> u32 {
        let mut b = 0;
        while b < self.superblock.size {
            let bp = bread(self.dev, BBLOCK!(b, &self.superblock));

            for bi in 0..BPB {
                if b + bi as u32 >= self.superblock.size {
                    break;
                }

                let m = 1 << (bi % 8);
                let byte_offset = (bi / 8) as usize;

                if ((*bp).data[byte_offset] & m) == 0 {
                    (*bp).data[byte_offset] |= m;
                    self.log.write(bp);
                    brelse(bp);
                    self.bzero(b + bi as u32);
                    return b + bi as u32;
                }
            }

            brelse(bp);
            b += BPB as u32;
        }

        println!("balloc: out of blocks");
        0
    }

    /// Free a disk block
    pub unsafe fn bfree(&mut self, b: u32) {
        let bp = bread(self.dev, BBLOCK!(b, &self.superblock));
        let bi = b % BPB as u32;
        let m = 1 << (bi % 8);
        let byte_offset = (bi / 8) as usize;

        if ((*bp).data[byte_offset] & m) == 0 {
            panic!("freeing free block");
        }

        (*bp).data[byte_offset] &= !m;
        self.log.write(bp);
        brelse(bp);
    }
}

/// Inode table cache
pub struct InodeTable {
    lock: SpinLock,
    inodes: [Inode; NINODE],
}

/// Global inode table
pub static mut ITABLE: InodeTable = InodeTable {
    lock: SpinLock::new("itable\0".as_bytes().as_ptr()),
    inodes: [Inode {
        dev: 0,
        inum: 0,
        ref_count: 0,
        lock: Sleeplock::new("inode\0".as_bytes().as_ptr()),
        valid: false,
        typ: 0,
        major: 0,
        minor: 0,
        nlink: 0,
        size: 0,
        addrs: [0; NDIRECT + 1],
    }; NINODE],
};

impl Inode {
    /// Create a new inode instance
    pub const fn new() -> Self {
        Self {
            dev: 0,
            inum: 0,
            ref_count: 0,
            lock: Sleeplock::new("inode\0".as_bytes().as_ptr()),
            valid: false,
            typ: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addrs: [0; NDIRECT + 1],
        }
    }

    /// Copy a modified in-memory inode to disk
    pub unsafe fn update(&mut self) {
        let bp = bread(self.dev, IBLOCK!(self.inum, &FS.superblock));
        let dip = ((*bp).data.as_mut_ptr() as *mut DiskInode).add(self.inum as usize % IPB);

        (*dip).type_ = self.typ;
        (*dip).major = self.major;
        (*dip).minor = self.minor;
        (*dip).nlink = self.nlink;
        (*dip).size = self.size;
        (*dip).addrs.copy_from_slice(&self.addrs);

        FS.log.write(bp);
        brelse(bp);
    }

    /// Get block address for file offset.
    /// Returns 0 if not allocated.
    pub unsafe fn bmap(&mut self, mut bn: u32) -> u32 {
        if bn < NDIRECT as u32 {
            if self.addrs[bn as usize] == 0 {
                let addr = FS.balloc();
                if addr != 0 {
                    self.addrs[bn as usize] = addr;
                }
            }
            return self.addrs[bn as usize];
        }

        bn -= NDIRECT as u32;
        if bn < NINDIRECT as u32 {
            // Load indirect block, allocating if necessary
            if self.addrs[NDIRECT] == 0 {
                let addr = FS.balloc();
                if addr != 0 {
                    self.addrs[NDIRECT] = addr;
                }
            }

            let bp = bread(self.dev, self.addrs[NDIRECT]);
            let indirect = (*bp).data.as_mut_ptr() as *mut [u32; NINDIRECT];

            if (*indirect)[bn as usize] == 0 {
                let addr = FS.balloc();
                if addr != 0 {
                    (*indirect)[bn as usize] = addr;
                    FS.log.write(bp);
                }
            }

            let addr = (*indirect)[bn as usize];
            brelse(bp);
            return addr;
        }

        panic!("bmap: out of range");
    }

    /// Truncate inode (discard contents).
    /// The inode must be locked.
    pub unsafe fn trunc(&mut self) {
        // Free direct blocks
        for i in 0..NDIRECT {
            if self.addrs[i] != 0 {
                FS.bfree(self.addrs[i]);
                self.addrs[i] = 0;
            }
        }

        // Free indirect blocks
        if self.addrs[NDIRECT] != 0 {
            let bp = bread(self.dev, self.addrs[NDIRECT]);
            let indirect = (*bp).data.as_ptr() as *const [u32; NINDIRECT];

            for i in 0..NINDIRECT {
                if (*indirect)[i] != 0 {
                    FS.bfree((*indirect)[i]);
                }
            }

            brelse(bp);
            FS.bfree(self.addrs[NDIRECT]);
            self.addrs[NDIRECT] = 0;
        }

        self.size = 0;
        self.update();
    }

    /// Copy stat information from inode.
    pub fn stati(&self, st: &mut Stat) {
        st.dev = self.dev as i32;
        st.ino = self.inum;
        st.typ = self.typ;
        st.nlink = self.nlink;
        st.size = self.size as u64;
    }

    /// Read data from inode.
    /// Returns number of bytes read.
    /// Returns -1 if error.
    pub unsafe fn readi(&mut self, user_dst: bool, mut dst: u64, mut off: u32, mut n: u32) -> i32 {
        if off > self.size || off.checked_add(n).is_none() {
            return 0;
        }

        if off + n > self.size {
            n = self.size - off;
        }

        let mut tot = 0;
        while tot < n {
            let addr = self.bmap(off / BSIZE as u32);
            if addr == 0 {
                break;
            }

            let bp = bread(self.dev, addr);
            let m = min(n - tot, BSIZE as u32 - off % BSIZE as u32);

            if either_copyout(
                user_dst,
                dst,
                (*bp).data.as_ptr().add(off as usize % BSIZE),
                m.into(),
            ) == -1
            {
                brelse(bp);
                return -1;
            }

            brelse(bp);
            tot += m;
            off += m;
            dst += m as u64;
        }

        tot as i32
    }

    /// Write data to inode.
    /// Returns number of bytes written.
    /// Returns -1 if error.
    pub unsafe fn writei(&mut self, user_src: bool, mut src: u64, mut off: u32, n: u32) -> i32 {
        if off > self.size || off.checked_add(n).is_none() {
            return -1;
        }

        let mut tot = 0;
        while tot < n {
            let addr = self.bmap(off / BSIZE as u32);
            if addr == 0 {
                break;
            }

            let bp = bread(self.dev, addr);
            let m = min(n - tot, BSIZE as u32 - off % BSIZE as u32);

            if either_copyin(
                (*bp).data.as_mut_ptr().add(off as usize % BSIZE),
                user_src,
                src,
                m.into(),
            ) == -1
            {
                brelse(bp);
                return -1;
            }

            FS.log.write(bp);
            brelse(bp);

            tot += m;
            off += m;
            src += m as u64;
        }

        if off > self.size {
            self.size = off;
        }

        self.update();
        tot as i32
    }
}

impl InodeTable {
    /// Initialize the inode table
    pub unsafe fn init(&mut self) {
        self.lock.initlock("itable\0".as_bytes().as_ptr());
        for inode in self.inodes.iter_mut() {
            inode.lock.init("inode\0".as_bytes().as_ptr());
        }
    }

    /// Allocate an inode on device dev.
    /// Mark it as allocated by giving it type typ.
    /// Returns an unlocked but allocated and referenced inode.
    pub unsafe fn alloc(&mut self, dev: u32, typ: i16) -> *mut Inode {
        for inum in 1..FS.superblock.ninodes {
            let bp = bread(dev, IBLOCK!(inum, &FS.superblock));
            let dip = ((*bp).data.as_ptr() as *mut DiskInode).add(inum as usize % IPB);

            if (*dip).type_ == 0 {
                // free inode
                ptr::write_bytes(dip as *mut u8, 0, core::mem::size_of::<DiskInode>());
                (*dip).type_ = typ;
                FS.log.write(bp); // mark it allocated on the disk
                brelse(bp);
                return self.get(dev, inum);
            }
            brelse(bp);
        }
        println!("ialloc: no inodes");
        ptr::null_mut()
    }

    /// Find the inode with number inum on device dev
    /// and return the in-memory copy. Does not lock
    /// the inode and does not read it from disk.
    pub unsafe fn get(&mut self, dev: u32, inum: u32) -> *mut Inode {
        self.lock.acquire();

        // Try to find existing inode
        if let Some(inode) = self
            .inodes
            .iter_mut()
            .find(|inode| inode.ref_count > 0 && inode.dev == dev && inode.inum == inum)
        {
            inode.ref_count += 1;
            self.lock.release();
            return inode as *mut Inode;
        }

        // Find an empty slot
        let inode = self
            .inodes
            .iter_mut()
            .find(|inode| inode.ref_count == 0)
            .expect("iget: no inodes");

        // Initialize new inode
        inode.dev = dev;
        inode.inum = inum;
        inode.ref_count = 1;
        inode.valid = false;

        self.lock.release();
        inode as *mut Inode
    }

    /// Increment reference count for inode
    pub unsafe fn dup(&mut self, inode: *mut Inode) -> *mut Inode {
        self.lock.acquire();
        (*inode).ref_count += 1;
        self.lock.release();
        inode
    }

    /// Drop a reference to an in-memory inode.
    /// If that was the last reference, the inode table entry can
    /// be recycled.
    /// If that was the last reference and the inode has no links
    /// to it, free the inode (and its content) on disk.
    pub unsafe fn put(&mut self, ip: *mut Inode) {
        self.lock.acquire();

        if (*ip).ref_count == 1 && (*ip).valid && (*ip).nlink == 0 {
            // inode has no links and no other references: truncate and free.
            (*ip).lock.acquire();
            self.lock.release();

            (*ip).trunc();
            (*ip).typ = 0;
            (*ip).update();
            (*ip).valid = false;

            (*ip).lock.release();
            self.lock.acquire();
        }

        (*ip).ref_count -= 1;
        self.lock.release();
    }

    /// Lock the given inode.
    /// Reads the inode from disk if necessary.
    pub unsafe fn lock(&mut self, ip: *mut Inode) {
        // Check input pointer and reference count
        if ip.is_null() || (*ip).ref_count < 1 {
            panic!("lock");
        }

        // Acquire the sleep lock
        (*ip).lock.acquire();

        // If inode not valid, read from disk
        if !(*ip).valid {
            // Read the disk block
            let bp = bread((*ip).dev, IBLOCK!((*ip).inum, &FS.superblock));
            let dip = ((*bp).data.as_ptr() as *const DiskInode).add((*ip).inum as usize % IPB);

            // Copy disk inode to in-memory inode
            (*ip).typ = (*dip).type_;
            (*ip).major = (*dip).major;
            (*ip).minor = (*dip).minor;
            (*ip).nlink = (*dip).nlink;
            (*ip).size = (*dip).size;
            (*ip).addrs.copy_from_slice(&(*dip).addrs);

            brelse(bp);
            (*ip).valid = true;

            if (*ip).typ == 0 {
                panic!("lock: no type");
            }
        }
    }

    /// Unlock the given inode.
    pub unsafe fn unlock(&mut self, ip: *mut Inode) {
        if ip.is_null() || !(*ip).lock.holding() || (*ip).ref_count < 1 {
            panic!("unlock");
        }

        (*ip).lock.release();
    }

    /// Common idiom: unlock, then put.
    pub unsafe fn unlockput(&mut self, ip: *mut Inode) {
        self.unlock(ip);
        self.put(ip);
    }
}
