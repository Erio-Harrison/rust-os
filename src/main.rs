#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub mod bio;
pub mod console;
pub mod elf;
pub mod file;
pub mod fs;
pub mod kalloc;
pub mod log;
pub mod memlayout;
pub mod param;
pub mod pipe;
pub mod proc;
pub mod riscv;
pub mod sbi;
pub mod sleeplock;
pub mod spinlock;
pub mod start;
pub mod stat;
pub mod string;
pub mod test;
pub mod test_device;
pub mod trap;
pub mod types;
pub mod uart;
pub mod virtio;
pub mod vm;

use core::arch::global_asm;
use core::panic::PanicInfo;

global_asm!(include_str!("arch/riscv/boot.S"));
global_asm!(include_str!("arch/riscv/trap.S"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello RISCV!");
    trap::init();

    println!("Testing breakpoint...");
    // 使用内联汇编直接插入 ebreak 指令
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack));
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
