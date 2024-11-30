#![no_std]
#![no_main]

mod vga_buffer;

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World{}", "!");

    use core::fmt::Write;
    vga_buffer::WRITER.lock().write_str("Hello-again").unwrap();
    write!(vga_buffer::WRITER.lock(),", some numbers: {}, {}",42,1.337).unwrap();

    //panic!("Some panic message");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}",_info);
    loop {}
}