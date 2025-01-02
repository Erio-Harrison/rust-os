#![no_std]
#![no_main]

use core::panic::PanicInfo;
pub mod sbi;
pub mod console;

use core::arch::global_asm;
global_asm!(include_str!("arch/riscv/boot.S"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("Hello RISCV!");
    panic!("END OF CODE");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n\nPanic: {}", info);
    loop {}
}