// src/test.rs

use crate::{print, println};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("\x1b[0;32m[kernel] Running {} tests\x1b[0m", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &core::panic::PanicInfo) -> ! {
    println!("\x1b[0;31m[failed]\x1b[0m");
    println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    let _ = exit_code;
    //crate::sbi::shutdown()
    //crate::test_device::exit_pass()
    match exit_code {
        QemuExitCode::Success => crate::test_device::exit_pass(),
        QemuExitCode::Failed => crate::test_device::exit_fail(),
    }
}

#[test_case]
fn test_println() {
    println!("test_println output");
}

#[test_case]
fn test_assertion() {
    assert_eq!(1 + 1, 2);
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for i in 0..5 {
        println!("test line {}", i);
    }
}
