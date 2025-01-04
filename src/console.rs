// src/println.rs

use core::fmt::{self, Write};
//use crate::sbi::console_putchar;
use crate::uart::putchar;

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            //console_putchar(c as usize);
            putchar(c as u8);
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

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    Stdout.write_fmt(args).unwrap();
}