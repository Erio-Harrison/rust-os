#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use rust_os::println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");
    rust_os::init();

    // println!("Timer interrupt test starting...");
    
    // loop {
    //     // 使用 hlt 指令让 CPU 进入低功耗状态
    //     // 直到下一个中断发生
    //     x86_64::instructions::hlt();
    // }

    // unsafe {
    //     *(0xdeadbeef as *mut u8) = 42;
    // }

    // x86_64::instructions::interrupts::int3();

    loop {
        use rust_os::print;
        print!("-");        // new
    }

    #[cfg(test)]
    test_main();

    println!("But nothing happened!");
    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}