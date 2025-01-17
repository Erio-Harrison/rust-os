use crate::param::*;
use super::spinlock::SpinLock;
use super::file::File;

// 保存的寄存器，用于内核上下文切换
#[repr(C)]
pub struct Context {
    pub ra: u64,
    pub sp: u64,
    // callee-saved
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

// CPU 状态
#[repr(C)]
pub struct Cpu {
    pub proc: *mut Proc,         // 当前在此CPU上运行的进程，或null
    pub context: Context,        // swtch() 切换到这里进入调度器
    pub noff: i32,              // push_off() 嵌套深度
    pub intena: bool,           // push_off()前中断是否启用
}

// trap帧，用于保存用户寄存器
#[repr(C)]
pub struct TrapFrame {
    pub kernel_satp: u64,    // 内核页表
    pub kernel_sp: u64,      // 进程内核栈顶
    pub kernel_trap: u64,    // usertrap()
    pub epc: u64,           // 保存的用户程序计数器
    pub kernel_hartid: u64,  // 保存的内核tp
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
}

// 进程状态
#[derive(Copy, Clone, PartialEq)]
pub enum ProcState {
    UNUSED,
    USED,
    SLEEPING,
    RUNNABLE,
    RUNNING,
    ZOMBIE,
}

// 每个进程的状态
#[repr(C)]
pub struct Proc {
    pub lock: SpinLock,
    
    // 使用这些字段时必须持有 p->lock:
    pub state: ProcState,
    pub chan: *mut u8,          // 如果非零，表示在 chan 上睡眠
    pub killed: i32,            // 如果非零，表示已被杀死
    pub xstate: i32,            // 退出状态，返回给父进程的wait
    pub pid: i32,               // 进程ID
    
    // 使用这个字段时必须持有 wait_lock:
    pub parent: *mut Proc,      // 父进程
    
    // 以下是进程私有的，不需要持有p->lock
    pub kstack: u64,            // 内核栈的虚拟地址
    pub sz: u64,                // 进程内存大小（字节）
    pub pagetable: *mut u64,    // 用户页表
    pub trapframe: *mut TrapFrame, // trampoline.S的数据页
    pub context: Context,       // swtch()切换到这里运行进程
    pub ofile: [*mut File; NOFILE], // 打开的文件
    pub cwd: *mut super::file::Inode,  // 当前目录
    pub name: [u8; 16],         // 进程名（用于调试）
}