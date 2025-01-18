use core::ptr;

use crate::memlayout::KSTACK;
use crate::riscv::PGSIZE;
use crate::{param::*, riscv};
use super::spinlock::SpinLock;
use super::file::File;

/// Registers saved for kernel context switches
#[repr(C)]
#[derive(Copy, Clone, Default)]  // Add these derives
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

/// Per-CPU state
#[repr(C)]
pub struct Cpu {
    pub proc: *mut Proc,         // The process running on this cpu, or null
    pub context: Context,        // swtch() here to enter scheduler()
    pub noff: i32,              // Depth of push_off() nesting
    pub intena: bool,           // Were interrupts enabled before push_off()
}

/// Trap frame, for saving user registers
#[repr(C)]
pub struct TrapFrame {
    pub kernel_satp: u64,    // kernel page table
    pub kernel_sp: u64,      // top of process's kernel stack
    pub kernel_trap: u64,    // usertrap()
    pub epc: u64,           // saved user program counter
    pub kernel_hartid: u64,  // saved kernel tp
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

/// Process states
#[derive(Copy, Clone, PartialEq)]
pub enum ProcState {
    UNUSED,
    USED,
    SLEEPING,
    RUNNABLE,
    RUNNING,
    ZOMBIE,
}

/// Per-process state
#[repr(C)]
pub struct Proc {
    pub lock: SpinLock,

    // p->lock must be held when using these:
    pub state: ProcState,
    pub chan: *mut u8,          // If non-zero, sleeping on chan
    pub killed: i32,            // If non-zero, have been killed
    pub xstate: i32,            // Exit status to be returned to parent's wait
    pub pid: i32,               // Process ID

    // wait_lock must be held when using this:
    pub parent: *mut Proc,      // Parent process

    // these are private to the process, so p->lock need not be held:
    pub kstack: u64,            // Virtual address of kernel stack
    pub sz: u64,                // Size of process memory (bytes)
    pub pagetable: *mut u64,    // User page table
    pub trapframe: *mut TrapFrame, // data page for trampoline.S
    pub context: Context,       // swtch() here to run process
    pub ofile: [*mut File; NOFILE], // Open files
    pub cwd: *mut super::file::Inode,  // Current directory
    pub name: [u8; 16],         // Process name (debugging)
}

// Instead of static initialization, we'll create an initialization function
static mut CPUS: [Cpu; NCPU] = unsafe { core::mem::zeroed() };
static mut PROCS: [Proc; NPROC] = unsafe { core::mem::zeroed() };

/// Initialize a new CPU state
fn cpu_init() -> Cpu {
    Cpu {
        proc: core::ptr::null_mut(),
        context: Context::default(),
        noff: 0,
        intena: false,
    }
}

/// Initialize a new process state
fn proc_init() -> Proc {
    Proc {
        lock: SpinLock::new(b"proc\0" as *const u8),
        state: ProcState::UNUSED,
        chan: core::ptr::null_mut(),
        killed: 0,
        xstate: 0,
        pid: 0,
        parent: core::ptr::null_mut(),
        kstack: 0,
        sz: 0,
        pagetable: core::ptr::null_mut(),
        trapframe: core::ptr::null_mut(),
        context: Context::default(),
        ofile: [core::ptr::null_mut(); NOFILE],
        cwd: core::ptr::null_mut(),
        name: [0; 16],
    }
}

// Process used for first user program
static mut INITPROC: *mut Proc = ptr::null_mut();

// Next process ID
static mut NEXTPID: i32 = 1;
static mut PID_LOCK: SpinLock = SpinLock::new(b"nextpid\0" as *const u8);

// Lock for synchronizing wait() operations
static mut WAIT_LOCK: SpinLock = SpinLock::new(b"wait_lock\0" as *const u8);

extern "C" {
    fn forkret();
    static trampoline: [u8; 0]; // trampoline.S
}

// /// Allocate a page for each process's kernel stack.
// /// Map it high in memory, followed by an invalid guard page.
// pub unsafe fn proc_mapstacks(kpgtbl: *mut u64) {
//     for i in 0..NPROC {
//         let pa = super::kalloc::kalloc();
//         if pa.is_null() {
//             panic!("kalloc");
//         }
//         let va = KSTACK(i);
//         super::vm::kvmmap(kpgtbl, va, pa as u64, PGSIZE as u64, PTE_R | PTE_W);
//     }
// }

/// Initialize the proc table.
pub unsafe fn procinit() {
    PID_LOCK.initlock(b"nextpid\0" as *const u8);
    WAIT_LOCK.initlock(b"wait_lock\0" as *const u8);
    
    for i in 0..NPROC {
        PROCS[i].lock.initlock(b"proc\0" as *const u8);
        PROCS[i].state = ProcState::UNUSED;
        PROCS[i].kstack = KSTACK(i);
    }
}

/// Must be called with interrupts disabled,
/// to prevent race with process being moved
/// to a different CPU.
#[inline]
pub unsafe fn cpuid() -> i32 {
    riscv::r_tp() as i32
}

/// Return this CPU's cpu struct.
/// Interrupts must be disabled.
#[inline]
pub unsafe fn mycpu() -> *mut Cpu {
    let id = cpuid();
    &mut CPUS[id as usize]
}

/// Return the current struct proc *, or zero if none.
#[inline]
pub unsafe fn myproc() -> *mut Proc {
    super::spinlock::push_off();
    let c = mycpu();
    let p = (*c).proc;
    super::spinlock::pop_off();
    p
}

/// Allocate a pid
unsafe fn allocpid() -> i32 {
    let mut pid;
    
    PID_LOCK.acquire();
    pid = NEXTPID;
    NEXTPID += 1;
    PID_LOCK.release();

    pid
}

/// Look in the process table for an UNUSED proc.
/// If found, initialize state required to run in the kernel,
/// and return with p->lock held.
/// If there are no free procs, or a memory allocation fails, return 0.
unsafe fn allocproc() -> *mut Proc {
    let mut p: *mut Proc;

    for i in 0..NPROC {
        p = &mut PROCS[i];
        (*p).lock.acquire();
        if (*p).state == ProcState::UNUSED {
            goto_found(p);
        } else {
            (*p).lock.release();
        }
    }
    return ptr::null_mut();

    fn goto_found(p: *mut Proc) -> *mut Proc {
        unsafe {
            (*p).pid = allocpid();
            (*p).state = ProcState::USED;

            // Allocate a trapframe page.
            (*p).trapframe = super::kalloc::kalloc() as *mut TrapFrame;
            if (*p).trapframe.is_null() {
                freeproc(p);
                (*p).lock.release();
                return ptr::null_mut();
            }

            // An empty user page table.
            // (*p).pagetable = super::vm::proc_pagetable(p);
            if (*p).pagetable.is_null() {
                freeproc(p);
                (*p).lock.release();
                return ptr::null_mut();
            }

            // Set up new context to start executing at forkret,
            // which returns to user space.
            (*p).context = Context::default();
            (*p).context.ra = forkret as u64;
            (*p).context.sp = (*p).kstack + PGSIZE as u64;

            p
        }
    }
}

/// Free a proc structure and the data hanging from it,
/// including user pages.
/// p->lock must be held.
unsafe fn freeproc(p: *mut Proc) {
    if !(*p).trapframe.is_null() {
        super::kalloc::kfree((*p).trapframe as *mut u8);
    }
    (*p).trapframe = ptr::null_mut();

    // if !(*p).pagetable.is_null() {
    //     super::vm::proc_freepagetable((*p).pagetable, (*p).sz);
    // }
    (*p).pagetable = ptr::null_mut();

    (*p).sz = 0;
    (*p).pid = 0;
    (*p).parent = ptr::null_mut();
    (*p).name[0] = 0;
    (*p).chan = ptr::null_mut();
    (*p).killed = 0;
    (*p).xstate = 0;
    (*p).state = ProcState::UNUSED;
}