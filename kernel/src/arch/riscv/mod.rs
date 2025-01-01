use core::arch::asm;

pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_BASE: usize = 0xffffffff80000000;

#[repr(C)]
pub struct TrapFrame {
    pub regs: [usize; 32],    // 通用寄存器
    pub sstatus: usize,       // 状态寄存器
    pub sepc: usize,          // 异常 PC
    pub stval: usize,         // 异常值
    pub scause: usize,        // 异常原因
}

pub fn init() {
    unsafe {
        // 设置中断向量表
        asm!("csrw stvec, {}", in(reg) trap_vector as usize);
        
        // 开启中断
        let mut sstatus: usize;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        sstatus |= 1 << 1;  // SIE bit
        asm!("csrw sstatus, {}", in(reg) sstatus);
    }
}

unsafe extern "C" {
    fn trap_vector();
}