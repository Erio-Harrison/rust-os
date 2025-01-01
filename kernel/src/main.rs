#![no_std]
#![no_main]

mod arch;
mod mm;
mod interrupt;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    // 初始化各个子系统
    mm::init();
    interrupt::init();
    
    println!("Kernel initialized!");
    
    loop {
        // 主循环
    }
}