use crate::fs::{BSIZE, FS, ITABLE, NDIRECT};
use crate::param::{MAXOPBLOCKS, NDEV, NFILE};
use crate::pipe::Pipe;
use crate::proc::myproc;
use crate::sleeplock::Sleeplock;
use crate::spinlock::SpinLock;
use crate::stat::Stat;

/// File types
pub const FD_NONE: i32 = 0; // Free file descriptor
pub const FD_PIPE: i32 = 1; // Pipe
pub const FD_INODE: i32 = 2; // Inode
pub const FD_DEVICE: i32 = 3; // Device

#[repr(C)]
#[derive(Clone, Copy)]
pub struct File {
    pub typ: i32,
    pub ref_count: i32,
    pub readable: bool,
    pub writable: bool,
    pub pipe: *mut Pipe,
    pub ip: *mut Inode,
    pub off: u32,
    pub major: i16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Inode {
    pub dev: u32,        // Device number
    pub inum: u32,       // Inode number
    pub ref_count: i32,  // Reference count
    pub lock: Sleeplock, // protects everything below here
    pub valid: bool,     // inode has been read from disk?
    pub typ: i16,
    pub major: i16,
    pub minor: i16,
    pub nlink: i16,
    pub size: u32,
    pub addrs: [u32; NDIRECT + 1],
}

/// File table
pub struct FileTable {
    lock: SpinLock,
    files: [File; NFILE],
}

/// Global file table
pub static mut FTABLE: FileTable = FileTable {
    lock: SpinLock::new("ftable\0".as_bytes().as_ptr()),
    files: [File {
        typ: FD_NONE,
        ref_count: 0,
        readable: false,
        writable: false,
        pipe: core::ptr::null_mut(),
        inode: core::ptr::null_mut(),
        offset: 0,
        major: 0,
    }; NFILE],
};

impl FileTable {
    /// Initialize the file table
    pub unsafe fn init(&mut self) {
        self.lock.initlock("ftable\0".as_bytes().as_ptr());
    }

    /// Allocate a file structure
    pub unsafe fn alloc(&mut self) -> Option<*mut File> {
        self.lock.acquire();

        // Find free file slot
        let f = self.files.iter_mut().find(|f| f.ref_count == 0).map(|f| {
            f.ref_count = 1;
            f as *mut File
        });

        self.lock.release();
        f
    }

    /// Increment ref count for file f
    pub unsafe fn dup(&mut self, f: *mut File) -> *mut File {
        self.lock.acquire();

        if (*f).ref_count < 1 {
            panic!("filedup");
        }

        (*f).ref_count += 1;
        self.lock.release();
        f
    }

    /// Close file f (Decrement ref count, close when reaches 0)
    pub unsafe fn close(&mut self, f: *mut File) {
        self.lock.acquire();

        if (*f).ref_count < 1 {
            panic!("fileclose");
        }

        if (*f).ref_count - 1 > 0 {
            (*f).ref_count -= 1;
            self.lock.release();
            return;
        }

        // Save file info
        let ff = *f;

        // Clear file table entry
        (*f).ref_count = 0;
        (*f).typ = FD_NONE;
        self.lock.release();

        match ff.typ {
            FD_PIPE => {
                // pipeclose(ff.pipe, ff.writable)
            }
            FD_INODE | FD_DEVICE => {
                FS.log.begin_op();
                ITABLE.put(ff.ip);
                FS.log.end_op();
            }
            _ => {}
        }
    }

    /// Read from file f.
    /// addr is a user virtual address.
    pub unsafe fn read(&mut self, f: *mut File, addr: u64, n: i32) -> i32 {
        if !(*f).readable {
            return -1;
        }

        match (*f).typ {
            FD_PIPE => {
                // piperead(f.pipe, addr, n)
                -1
            }
            FD_DEVICE => {
                if (*f).major < 0
                    || (*f).major as usize >= NDEV
                    || devsw[(*f).major as usize].read.is_none()
                {
                    return -1;
                }
                devsw[(*f).major as usize].read.unwrap()(1, addr, n)
            }
            FD_INODE => {
                ITABLE.lock((*f).ip);
                let r = readi((*f).inode, true, addr, (*f).offset, n as u32);
                if r > 0 {
                    (*f).offset += r as u32;
                }
                ITABLE.unlock((*f).inode);
                r
            }
            _ => panic!("fileread"),
        }
    }

    /// Write to file f.
    /// addr is a user virtual address.
    pub unsafe fn write(&mut self, f: *mut File, addr: u64, n: i32) -> i32 {
        if !(*f).writable {
            return -1;
        }

        match (*f).typ {
            FD_PIPE => {
                // pipewrite(f.pipe, addr, n)
                -1
            }
            FD_DEVICE => {
                if (*f).major < 0
                    || (*f).major as usize >= NDEV
                    || devsw[(*f).major as usize].write.is_none()
                {
                    return -1;
                }
                devsw[(*f).major as usize].write.unwrap()(1, addr, n)
            }
            FD_INODE => {
                // Maximum write size
                let max = ((MAXOPBLOCKS - 4) / 2) * BSIZE;
                let mut i = 0;
                let mut ret = 0;

                while i < n {
                    let n1 = core::cmp::min(n - i, max as i32);

                    FS.log.begin_op();
                    ITABLE.lock((*f).ip);

                    let r = writei((*f).ip, true, addr + i as u64, (*f).offset, n1 as u32);
                    if r > 0 {
                        (*f).offset += r as u32;
                    }

                    ITABLE.unlock((*f).ip);
                    FS.log.end_op();

                    if r != n1 {
                        break;
                    }

                    i += r;
                }

                if i == n {
                    n
                } else {
                    -1
                }
            }
            _ => panic!("filewrite"),
        }
    }

    /// Get metadata about file f.
    pub unsafe fn stat(&mut self, f: *mut File, addr: u64) -> i32 {
        let p = myproc();
        let mut st = Stat::new();

        match (*f).typ {
            FD_INODE | FD_DEVICE => {
                ITABLE.lock((*f).ip);
                stati((*f).ip, &mut st);
                ITABLE.unlock((*f).ip);

                if copyout(
                    (*p).pagetable,
                    addr,
                    &st as *const _ as *const u8,
                    core::mem::size_of::<Stat>(),
                ) < 0
                {
                    return -1;
                }
                0
            }
            _ => -1,
        }
    }
}
