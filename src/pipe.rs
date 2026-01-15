use crate::kalloc::{kalloc, kfree};
use crate::proc::{either_copyin, either_copyout, myproc, sleep, wakeup};
use crate::spinlock::SpinLock;
use core::ptr;

pub const PIPESIZE: usize = 512;

#[repr(C)]
pub struct Pipe {
    pub lock: SpinLock,
    pub data: [u8; PIPESIZE],
    pub nread: u32,  // number of bytes read
    pub nwrite: u32, // number of bytes written
    pub readopen: bool,  // read fd is still open
    pub writeopen: bool, // write fd is still open
}

impl Pipe {
    /// Allocate a new pipe
    pub unsafe fn alloc() -> Option<*mut Pipe> {
        let pa = kalloc();
        if pa.is_null() {
            return None;
        }

        let pi = pa as *mut Pipe;
        (*pi).lock = SpinLock::new(b"pipe\0".as_ptr());
        (*pi).nread = 0;
        (*pi).nwrite = 0;
        (*pi).readopen = true;
        (*pi).writeopen = true;
        ptr::write_bytes((*pi).data.as_mut_ptr(), 0, PIPESIZE);

        Some(pi)
    }

    /// Close the pipe (either read or write end)
    pub unsafe fn close(&mut self, writable: bool) {
        self.lock.acquire();

        if writable {
            self.writeopen = false;
            wakeup(self as *mut Pipe as *mut u8);
        } else {
            self.readopen = false;
            wakeup(self as *mut Pipe as *mut u8);
        }

        // Free pipe if both ends are closed
        if !self.readopen && !self.writeopen {
            self.lock.release();
            kfree(self as *mut Pipe as *mut u8);
        } else {
            self.lock.release();
        }
    }

    /// Write to pipe
    pub unsafe fn write(&mut self, addr: u64, n: i32) -> i32 {
        let mut i = 0;
        let p = myproc();

        self.lock.acquire();

        while i < n {
            // Check if pipe is still readable
            if !self.readopen || (*p).killed != 0 {
                self.lock.release();
                return -1;
            }

            // If pipe is full, wait for reader
            if self.nwrite == self.nread + PIPESIZE as u32 {
                wakeup(self as *mut Pipe as *mut u8);
                sleep(self as *mut Pipe as *mut u8, &mut self.lock);
            } else {
                // Write one byte to pipe buffer
                let mut ch: u8 = 0;
                if either_copyin(&mut ch as *mut u8, true, addr + i as u64, 1) == -1 {
                    break;
                }
                self.data[(self.nwrite % PIPESIZE as u32) as usize] = ch;
                self.nwrite += 1;
                i += 1;
            }
        }

        wakeup(self as *mut Pipe as *mut u8);
        self.lock.release();

        i
    }

    /// Read from pipe
    pub unsafe fn read(&mut self, addr: u64, n: i32) -> i32 {
        let p = myproc();

        self.lock.acquire();

        // Wait while pipe is empty and write end is still open
        while self.nread == self.nwrite && self.writeopen {
            if (*p).killed != 0 {
                self.lock.release();
                return -1;
            }
            sleep(self as *mut Pipe as *mut u8, &mut self.lock);
        }

        // Read bytes from pipe buffer
        let mut i = 0;
        while i < n {
            if self.nread == self.nwrite {
                break;
            }
            let ch = self.data[(self.nread % PIPESIZE as u32) as usize];
            self.nread += 1;
            if either_copyout(true, addr + i as u64, &ch as *const u8, 1) == -1 {
                break;
            }
            i += 1;
        }

        wakeup(self as *mut Pipe as *mut u8);
        self.lock.release();

        i
    }
}
