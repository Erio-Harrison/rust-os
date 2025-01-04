// src/uart.rs

//! UART 控制器驱动
//! QEMU RISC-V Virt 机器的 UART 控制器是 NS16550A

use core::ptr::{read_volatile, write_volatile};

// QEMU virt 机器的 UART 物理地址
const UART_BASE_ADDR: usize = 0x1000_0000;

// UART 相关寄存器偏移
const RBR: usize = 0x0; // 接收缓存，读
const THR: usize = 0x0; // 发送保持，写
const DLL: usize = 0x0; // 除数锁存低字节
const DLM: usize = 0x1; // 除数锁存高字节
const IER: usize = 0x1; // 中断使能
const FCR: usize = 0x2; // FIFO 控制
const LCR: usize = 0x3; // 线路控制
const LSR: usize = 0x5; // 线路状态

// 线路状态寄存器相关位
const LSR_DR: u8 = 1 << 0;   // 数据就绪
const LSR_THRE: u8 = 1 << 5; // THR 空

/// 初始化 UART
pub fn init() {
    unsafe {
        // 关闭中断
        write_reg(IER, 0x00);

        // 设置波特率
        write_reg(LCR, 0x80); // 设置 DLAB 位，允许设置波特率
        write_reg(DLL, 0x03); // 设置除数为 3，波特率为 38.4K
        write_reg(DLM, 0x00);

        // 配置传输格式: 8 位数据位，1 位停止位，无校验
        write_reg(LCR, 0x03);

        // 启用 FIFO，清空 FIFO
        write_reg(FCR, 0x07);

        // 启用中断
        write_reg(IER, 0x01);
    }
}

/// 写入一个字节
pub fn putchar(c: u8) {
    unsafe {
        // 等待发送保持寄存器为空
        while (read_reg(LSR) & LSR_THRE) == 0 {}
        write_reg(THR, c);
    }
}

/// 读取一个字节
pub fn getchar() -> Option<u8> {
    unsafe {
        if (read_reg(LSR) & LSR_DR) == 0 {
            None
        } else {
            Some(read_reg(RBR))
        }
    }
}

/// 从寄存器读取
unsafe fn read_reg(reg: usize) -> u8 {
    read_volatile((UART_BASE_ADDR + reg) as *const u8)
}

/// 写入寄存器
unsafe fn write_reg(reg: usize, val: u8) {
    write_volatile((UART_BASE_ADDR + reg) as *mut u8, val)
}