#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use core::panic::PanicInfo;
use rust_os::{allocator, memory::BootInfoFrameAllocator, println};
use bootloader::BootInfo;
use x86_64::{structures::paging::{Page, PageTable}, VirtAddr};
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

#[no_mangle]
pub extern "C" fn _start(boot_info : &'static BootInfo) -> ! {
    use rust_os::memory;
    use x86_64::{structures::paging::Translate, VirtAddr};
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

    // Note: The actual address might be different for you. Use the address that
    // your page fault handler reports.
    // let ptr = 0x2031b2 as *mut u8;

    // // read from a code page
    // unsafe { let x = *ptr; }
    // println!("read worked");

    // // write to a code page
    // unsafe { *ptr = 42; }
    // println!("write worked");

    // use rust_os::memory::active_level_4_table;
    // use x86_64::VirtAddr;

    // let (level_4_page_table, _) = Cr3::read();
    // println!("Level 4 page table at: {:?}", level_4_page_table.start_address());
    // let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // let l4_table = unsafe { active_level_4_table(phys_mem_offset) };

    // for (i, entry) in l4_table.iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("L4 Entry {}: {:?}", i, entry);
    //     }
    // }

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    //let mut frame_allocator = memory::EmptyFramAllocator;

    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // map an unused page
    //let page = Page::containing_address(VirtAddr::new(0));
    let page2 = Page::containing_address(VirtAddr::new(0xdeadbeaf000));

    memory::create_example_mapping(page2, &mut mapper, &mut frame_allocator);

    // write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page2.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

    // let addresses = [
    //     // the identity-mapped vga buffer page
    //     0xb8000,
    //     // some code page
    //     0x201008,
    //     // some stack page
    //     0x0100_0020_1a10,
    //     // virtual address mapped to physical address 0
    //     boot_info.physical_memory_offset,
    // ];

    // for &address in &addresses {
    //     let virt = VirtAddr::new(address);
    //     let phys = mapper.translate_addr(virt);
    //     println!("{:?} -> {:?}", virt, phys);
    // }
    allocator::init_heap(&mut mapper, &mut frame_allocator)
    .expect("heap initialization failed");

    let heap_value = Box::new(1);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));    

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