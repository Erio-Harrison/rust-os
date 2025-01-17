#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod console;
pub mod sbi;
pub mod test_device;
pub mod test;
pub mod uart;
pub mod trap;
pub mod spinlock;
pub mod proc;
pub mod param;
pub mod types;
pub mod file;

use core::panic::PanicInfo;
use core::arch::global_asm;

global_asm!(include_str!("arch/riscv/boot.S"));
global_asm!(include_str!("arch/riscv/trap.S"));


#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello RISCV!");
    trap::init();

    println!("Testing breakpoint...");
    // 使用内联汇编直接插入 ebreak 指令
    unsafe {
        core::arch::asm!(
            "ebreak",
            options(nomem, nostack)
        );
    }

    println!("Successfully handled breakpoint!");
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