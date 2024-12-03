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
    use x86_64::registers::control::Cr3;

    let (level_4_page_table, _) = Cr3::read();
    println!("Level 4 page table at: {:?}", level_4_page_table.start_address());
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

    // Note: The actual address might be different for you. Use the address that
    // your page fault handler reports.
    // let ptr = 0x2031b2 as *mut u8;

    // // read from a code page
    // unsafe { let x = *ptr; }
    // println!("read worked");

    // // write to a code page
    // unsafe { *ptr = 42; }
    // println!("write worked");

    #[cfg(test)]
    test_main();

    // loop {
    //     use rust_os::print;
    //     print!("-"); 
    // }

    println!("But nothing happened!");
    rust_os::hlt_loop();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    rust_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rust_os::test_panic_handler(info)
}