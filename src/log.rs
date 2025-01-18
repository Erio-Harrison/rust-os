//! Log module for filesystem transactions
use crate::{
    bio::*,
    fs::{Superblock, BSIZE},
    param::{LOGSIZE, MAXOPBLOCKS},
    proc::{sleep, wakeup},
    spinlock::SpinLock,
    virtio::Buffer,
};
use core::mem::size_of;

/// On-disk log header
#[repr(C)]
#[derive(Clone)]
struct LogHeader {
    n: i32,                // Number of log blocks
    block: [i32; LOGSIZE], // Block numbers for each log block
}

/// Log structure for both memory and disk
#[derive(Clone)]
pub struct Log {
    lock: SpinLock,   // Protects the log
    start: i32,       // First log block
    size: i32,        // Number of log blocks
    outstanding: i32, // How many FS sys calls are executing
    committing: i32,  // In commit(), please wait
    dev: i32,         // Device number
    lh: LogHeader,    // In-memory log header
}

impl Log {
    /// Create a new log instance
    pub const fn new() -> Self {
        Self {
            lock: SpinLock::new("log\0".as_bytes().as_ptr()),
            start: 0,
            size: 0,
            outstanding: 0,
            committing: 0,
            dev: 0,
            lh: LogHeader {
                n: 0,
                block: [0; LOGSIZE],
            },
        }
    }

    /// Initialize the log
    pub unsafe fn init(&mut self, dev: i32, sb: &Superblock) {
        if size_of::<LogHeader>() >= BSIZE {
            panic!("initlog: too big logheader");
        }

        self.lock.initlock("log\0".as_bytes().as_ptr());
        self.start = sb.logstart as i32;
        self.size = sb.nlog as i32;
        self.dev = dev;
        self.recover_from_log();
    }

    /// Copy committed blocks from log to their home location
    unsafe fn install_trans(&self, recovering: i32) {
        for tail in 0..self.lh.n {
            let lbuf = bread(self.dev as u32, (self.start + tail + 1) as u32);
            let dbuf = bread(self.dev as u32, self.lh.block[tail as usize] as u32);
            (*dbuf).data.copy_from_slice(&(*lbuf).data);
            bwrite(dbuf);

            if recovering == 0 {
                bunpin(dbuf);
            }
            brelse(lbuf);
            brelse(dbuf);
        }
    }

    /// Read the log header from disk
    unsafe fn read_head(&mut self) {
        let buf = bread(self.dev as u32, self.start as u32);
        let lh = (*buf).data.as_ptr() as *const LogHeader;

        self.lh.n = (*lh).n;
        for i in 0..self.lh.n as usize {
            self.lh.block[i] = (*lh).block[i];
        }

        brelse(buf);
    }

    /// Write in-memory log header to disk
    unsafe fn write_head(&self) {
        let buf = bread(self.dev as u32, self.start as u32);
        let hb = (*buf).data.as_mut_ptr() as *mut LogHeader;

        (*hb).n = self.lh.n;
        for i in 0..self.lh.n as usize {
            (*hb).block[i] = self.lh.block[i];
        }

        bwrite(buf);
        brelse(buf);
    }

    /// Recover from log
    unsafe fn recover_from_log(&mut self) {
        self.read_head();
        self.install_trans(1);
        self.lh.n = 0;
        self.write_head();
    }

    /// Write modified blocks from cache to log
    unsafe fn write_log(&self) {
        for tail in 0..self.lh.n {
            let to = bread(self.dev as u32, (self.start + tail + 1) as u32);
            let from = bread(self.dev as u32, self.lh.block[tail as usize] as u32);
            (*to).data.copy_from_slice(&(*from).data);
            bwrite(to);
            brelse(from);
            brelse(to);
        }
    }

    /// Begin a transaction
    pub unsafe fn begin_op(&mut self) {
        loop {
            self.lock.acquire();

            if self.committing != 0 {
                sleep(self as *mut _ as *mut u8, &mut self.lock);
            } else if self.lh.n + (self.outstanding + 1) * MAXOPBLOCKS as i32 > LOGSIZE as i32 {
                sleep(self as *mut _ as *mut u8, &mut self.lock);
            } else {
                self.outstanding += 1;
                self.lock.release();
                break;
            }
        }
    }

    /// End a transaction
    pub unsafe fn end_op(&mut self) {
        let mut do_commit = false;

        self.lock.acquire();
        self.outstanding -= 1;

        if self.committing != 0 {
            panic!("log.committing");
        }

        if self.outstanding == 0 {
            do_commit = true;
            self.committing = 1;
        } else {
            wakeup(self as *mut _ as *mut u8);
        }

        self.lock.release();

        if do_commit {
            self.commit();
            self.lock.acquire();
            self.committing = 0;
            wakeup(self as *mut _ as *mut u8);
            self.lock.release();
        }
    }

    /// Commit current transaction
    unsafe fn commit(&mut self) {
        if self.lh.n > 0 {
            self.write_log();
            self.write_head();
            self.install_trans(0);
            self.lh.n = 0;
            self.write_head();
        }
    }

    /// Write a block to the log
    pub unsafe fn write(&mut self, b: *mut Buffer) {
        self.lock.acquire();

        if self.lh.n >= LOGSIZE as i32 || self.lh.n >= self.size - 1 {
            panic!("too big a transaction");
        }
        if self.outstanding < 1 {
            panic!("log_write outside of trans");
        }

        let mut i = 0;
        while i < self.lh.n {
            if self.lh.block[i as usize] == (*b).blockno as i32 {
                break;
            }
            i += 1;
        }

        self.lh.block[i as usize] = (*b).blockno as i32;
        if i == self.lh.n {
            bpin(b);
            self.lh.n += 1;
        }

        self.lock.release();
    }
}
