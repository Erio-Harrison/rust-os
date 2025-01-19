use core::fmt;

use crate::{file::{CONSOLE, DEVSW},
 proc::{either_copyin, either_copyout, killed, myproc, procdump, sleep, wakeup}, 
 spinlock::SpinLock, uart::{ uartputc, uartputc_sync}};

const BACKSPACE: i32 = 0x100;
const INPUT_BUF_SIZE: usize = 128;

fn ctrl(x: u8) -> u8 {
    x - b'@'
}

pub static mut CONS: Console = Console::new();

pub struct Console {
    lock: SpinLock,
    buf: [u8; INPUT_BUF_SIZE],
    r: usize,  // Read index
    w: usize,  // Write index
    e: usize,  // Edit index
}

impl Console {
    pub const fn new() -> Self {
        Self {
            lock: SpinLock::new("cons\0".as_bytes().as_ptr()),
            buf: [0; INPUT_BUF_SIZE],
            r: 0,
            w: 0,
            e: 0,
        }
    }

    /// Output a character to the console
    pub unsafe fn consputc(&self, c: i32) {
        if c == BACKSPACE {
            uartputc_sync(b'b'); 
            uartputc_sync(b' '); 
            uartputc_sync(b'b');
        } else {

        }
    }

    /// User write to the console
    pub unsafe fn consolewrite(&mut self, user_src: bool, src: u64, n: usize) -> i32 {
        let mut i = 0;
        while i < n {
            let mut c: u8 = 0;
            if either_copyin(&mut c as *mut u8, user_src, src + i as u64, 1) == -1 {
                break;
            }
            uartputc(c);
            i += 1;
        }
        i as i32
    }

    /// User read from the console
    pub unsafe fn consoleread(&mut self, user_dst: bool, dst: u64, n: usize) -> i32 {
        let target = n;
        let mut n = n;
        
        self.lock.acquire();
        
        while n > 0 {
            // Wait until interrupt handler has put some input into cons.buffer
            while self.r == self.w {
                if killed(myproc()) {
                    self.lock.release();
                    return -1;
                }
                sleep(&self.r as *const _ as *mut u8, &mut self.lock);
            }

            let c = self.buf[self.r % INPUT_BUF_SIZE];
            self.r += 1;

            if c == ctrl(b'D') {  // End-of-file
                if n < target {
                    // Save ^D for next time
                    self.r -= 1;
                }
                break;
            }

            // Copy the input byte to the user-space buffer
            if either_copyout(user_dst, dst + (target - n) as u64, &c, 1) == -1 {
                break;
            }

            n -= 1;

            if c == b'\n' {  // A whole line has arrived
                break;
            }
        }

        self.lock.release();
        (target - n) as i32
    }

    /// The console input interrupt handler
    pub unsafe fn consoleintr(&mut self, c: i32) {
        self.lock.acquire();

        match c as u8 {
            c if c == ctrl(b'P') => {  // Print process list
                procdump();
            }
            c if c == ctrl(b'U') => {  // Kill line
                while self.e != self.w && 
                    self.buf[(self.e - 1) % INPUT_BUF_SIZE] != b'\n' {
                    self.e -= 1;
                    self.consputc(BACKSPACE);
                }
            }
            c if c == ctrl(b'H') || c == 0x7f => {  // Backspace
                if self.e != self.w {
                    self.e -= 1;
                    self.consputc(BACKSPACE);
                }
            }
            _ => {
                if c != 0 && (self.e - self.r) < INPUT_BUF_SIZE {
                    let c = if c == '\r' as i32 { '\n' as i32 } else { c };
                    
                    // Echo back to the user
                    self.consputc(c);

                    // Store for consumption by consoleread()
                    self.buf[self.e % INPUT_BUF_SIZE] = c as u8;
                    self.e += 1;

                    if c as u8 == b'\n' || c as u8 == ctrl(b'D') || 
                        self.e - self.r == INPUT_BUF_SIZE {
                        // Wake up consoleread() if a whole line has arrived
                        self.w = self.e;
                        wakeup(&self.r as *const _ as *mut u8);
                    }
                }
            }
        }

        self.lock.release();
    }
}

/// Print formatting implementation
pub struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            unsafe { uartputc_sync(c as u8) };
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut stdout = $crate::console::Stdout;
        stdout.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Initialize the console
pub unsafe fn consoleinit() {
    CONS.lock.initlock("cons\0".as_bytes().as_ptr());
    
    // Initialize UART
    crate::uart::uartinit();

    unsafe fn console_read(user_dst: i32, dst: u64, n: i32) -> i32 {
        CONS.consoleread(user_dst != 0, dst, n as usize)
    }

    unsafe fn console_write(user_src: i32, src: u64, n: i32) -> i32 {
        CONS.consolewrite(user_src != 0, src, n as usize)
    }

    // 连接读写系统调用
    DEVSW[CONSOLE].read = Some(console_read);
    DEVSW[CONSOLE].write = Some(console_write);
}