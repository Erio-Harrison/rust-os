mod page_table;
use page_table::{PageTable, PageTableEntry, PageTableFlags};

pub fn init() {
    // 初始化内核页表
    init_kernel_page_table();
}

fn init_kernel_page_table() {
    let kernel_table = unsafe { &mut *(boot_page_table as *mut PageTable) };
    
    // 映射内核段
    let flags = PageTableFlags::VALID 
        | PageTableFlags::READABLE 
        | PageTableFlags::WRITABLE 
        | PageTableFlags::EXECUTABLE;
        
    // 恒等映射前 1GB 物理内存
    for i in 0..256 {
        let addr = i * 0x200000;
        kernel_table.entries[i] = PageTableEntry::new(addr, flags);
    }
    
    // 映射内核到高地址
    let kernel_offset = KERNEL_BASE >> 30;
    for i in 0..256 {
        let addr = i * 0x200000;
        kernel_table.entries[kernel_offset + i] = PageTableEntry::new(addr, flags);
    }
}

extern "C" {
    static boot_page_table: usize;
}

// kernel/src/interrupt/mod.rs
use crate::arch::TrapFrame;

pub fn init() {
    // 初始化中断描述符表
    arch::init();
}

#[no_mangle]
pub extern "C" fn trap_handler(tf: &mut TrapFrame) {
    match tf.scause {
        // 软件中断
        1 => handle_software_interrupt(tf),
        // 时钟中断
        5 => handle_timer_interrupt(tf),
        // 外部中断
        9 => handle_external_interrupt(tf),
        // 缺页异常
        12 | 13 | 15 => handle_page_fault(tf),
        // 系统调用
        8 => handle_syscall(tf),
        _ => panic!("Unhandled trap: {}", tf.scause),
    }
}

fn handle_software_interrupt(tf: &mut TrapFrame) {
    // 处理软件中断
}

fn handle_timer_interrupt(tf: &mut TrapFrame) {
    // 处理时钟中断
}

fn handle_external_interrupt(tf: &mut TrapFrame) {
    // 处理外部中断
}

fn handle_page_fault(tf: &mut TrapFrame) {
    // 处理缺页异常
}

fn handle_syscall(tf: &mut TrapFrame) {
    // 处理系统调用
}
