#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod println;
pub mod sbi;
pub mod test;
pub mod uart;

use core::panic::PanicInfo;
use core::arch::global_asm;

global_asm!(include_str!("arch/riscv/boot.S"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello RISCV!");
    
    #[cfg(test)]
    test_main();
    
    println!("It did not crash!");
    loop {}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n\nPanic: {}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test::test_panic_handler(info)
}

// 测试用例
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