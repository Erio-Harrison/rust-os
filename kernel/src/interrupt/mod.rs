use crate::arch::{self, TrapFrame};

pub fn init() {
    // 初始化中断描述符表
    arch::init();
}

#[unsafe(no_mangle)]
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