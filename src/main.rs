#![no_std]
#![no_main]

use core::panic::PanicInfo;
pub mod sbi;

use core::arch::global_asm;
global_asm!(include_str!("arch/riscv/boot.S"));

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    let hello = b"Hello, world!\n";
    for &c in hello.iter() {
        sbi::console_putchar(c as usize);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}