#![no_std]  
#![feature(panic_info_message)] 

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}