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
pub mod riscv_local;
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
pub mod plic;
pub mod syscall;

use core::arch::global_asm;
use core::panic::PanicInfo;
use proc::scheduler;
use riscv_local::r_mhartid;

use core::sync::atomic::{AtomicBool, Ordering};

static STARTED: AtomicBool = AtomicBool::new(false);



global_asm!(include_str!("arch/riscv/macro.S"));

global_asm!(include_str!("arch/riscv/boot.S"));
global_asm!(include_str!("arch/riscv/kernelvec.S"));
global_asm!(include_str!("arch/riscv/trampoline.S"));
global_asm!(include_str!("arch/riscv/swtch.S"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    unsafe { 
        crate::uart::uartinit();
        crate::console::consoleinit();
    }
    println!("xv6 kernel is booting");

    let hart_id = unsafe { r_mhartid() };
    if hart_id == 0 {
        kernel_init();
        set_started(true);
    } else {
        while !get_started() {}
        println!("Hart {} starting", hart_id);
        hart_init();
    }

    unsafe { scheduler() };
}

fn set_started(started: bool) {
    STARTED.store(started, Ordering::SeqCst);
}

fn get_started() -> bool {
    STARTED.load(Ordering::SeqCst)
}

fn kernel_init() {
    unsafe { crate::console::consoleinit() };
    println!("Console initialized.");
    unsafe { crate::trap::trapinit() };
    crate::proc::proc_init();
    println!("Kernel initialized.");
}


fn hart_init() {
    unsafe { crate::trap::trapinithart() };
    unsafe { crate::plic::plicinithart() };
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
