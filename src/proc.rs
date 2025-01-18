use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use super::file::File;
use super::spinlock::SpinLock;
use crate::memlayout::{KSTACK, TRAMPOLINE, TRAPFRAME};
use crate::riscv::{intr_get, intr_on, PageTable, PGSIZE, PTE_R, PTE_W, PTE_X};
use crate::types::uint;
use crate::vm::{
    copyin, copyout, mappages, uvmalloc, uvmcopy, uvmcreate, uvmdealloc, uvmfirst, uvmfree,
    uvmunmap,
};
use crate::{param::*, println, riscv};

/// Registers saved for kernel context switches
#[repr(C)]
#[derive(Copy, Clone, Default)] // Add these derives
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
    pub proc: *mut Proc,  // The process running on this cpu, or null
    pub context: Context, // swtch() here to enter scheduler()
    pub noff: i32,        // Depth of push_off() nesting
    pub intena: bool,     // Were interrupts enabled before push_off()
}

/// Trap frame, for saving user registers
#[repr(C)]
pub struct TrapFrame {
    pub kernel_satp: u64,   // kernel page table
    pub kernel_sp: u64,     // top of process's kernel stack
    pub kernel_trap: u64,   // usertrap()
    pub epc: u64,           // saved user program counter
    pub kernel_hartid: u64, // saved kernel tp
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
    pub chan: *mut u8, // If non-zero, sleeping on chan
    pub killed: i32,   // If non-zero, have been killed
    pub xstate: i32,   // Exit status to be returned to parent's wait
    pub pid: i32,      // Process ID

    // wait_lock must be held when using this:
    pub parent: *mut Proc, // Parent process

    // these are private to the process, so p->lock need not be held:
    pub kstack: u64,                  // Virtual address of kernel stack
    pub sz: u64,                      // Size of process memory (bytes)
    pub pagetable: *mut u64,          // User page table
    pub trapframe: *mut TrapFrame,    // data page for trampoline.S
    pub context: Context,             // swtch() here to run process
    pub ofile: [*mut File; NOFILE],   // Open files
    pub cwd: *mut super::file::Inode, // Current directory
    pub name: [u8; 16],               // Process name (debugging)
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

/// Allocate a page for each process's kernel stack.
/// Map it high in memory, followed by an invalid guard page.
pub unsafe fn proc_mapstacks(kpgtbl: *mut usize) {
    for i in 0..NPROC {
        let pa = super::kalloc::kalloc();
        if pa.is_null() {
            panic!("kalloc");
        }
        let va = KSTACK(i);
        super::vm::kvmmap(
            kpgtbl,
            va,
            pa as u64,
            PGSIZE as u64,
            (PTE_R | PTE_W).try_into().unwrap(),
        );
    }
}

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

/// Create a user page table for a given process, with no user memory,
/// but with trampoline and trapframe pages.
pub unsafe fn proc_pagetable(p: *mut Proc) -> PageTable {
    // An empty page table
    let pagetable = uvmcreate();
    if pagetable.is_null() {
        return ptr::null_mut();
    }

    // map the trampoline code (for system call return)
    // at the highest user virtual address.
    // only the supervisor uses it, on the way
    // to/from user space, so not PTE_U.
    if mappages(
        pagetable,
        TRAMPOLINE,
        PGSIZE as u64,
        &TRAMPOLINE as *const _ as u64,
        PTE_R | PTE_X,
    ) < 0
    {
        uvmfree(pagetable, 0);
        return ptr::null_mut();
    }

    // map the trapframe page just below the trampoline page, for
    // trampoline.S.
    if mappages(
        pagetable,
        TRAPFRAME,
        PGSIZE as u64,
        (*p).trapframe as u64,
        PTE_R | PTE_W,
    ) < 0
    {
        uvmunmap(pagetable, TRAMPOLINE, 1, false);
        uvmfree(pagetable, 0);
        return ptr::null_mut::<u64>();
    }

    pagetable as *mut u64
}

/// Free a process's page table, and free the
/// physical memory it refers to.
pub unsafe fn proc_freepagetable(pagetable: PageTable, sz: u64) {
    uvmunmap(pagetable as *mut usize, TRAMPOLINE, 1, false);
    uvmunmap(pagetable as *mut usize, TRAPFRAME, 1, false);
    uvmfree(pagetable as *mut usize, sz);
}

/// A user program that calls exec("/init")
/// assembled from ../user/initcode.S
/// od -t xC ../user/initcode
pub static INITCODE: &[u8] = &[
    0x17, 0x05, 0x00, 0x00, 0x13, 0x05, 0x45, 0x02, 0x97, 0x05, 0x00, 0x00, 0x93, 0x85, 0x35, 0x02,
    0x93, 0x08, 0x70, 0x00, 0x73, 0x00, 0x00, 0x00, 0x93, 0x08, 0x20, 0x00, 0x73, 0x00, 0x00, 0x00,
    0xef, 0xf0, 0x9f, 0xff, 0x2f, 0x69, 0x6e, 0x69, 0x74, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

/// Set up first user process.
pub unsafe fn userinit() {
    // Allocate first process
    let p = allocproc();
    INITPROC = p;

    // allocate one user page and copy initcode's instructions
    // and data into it.
    uvmfirst(
        (*p).pagetable as *mut usize,
        INITCODE.as_ptr(),
        INITCODE.len(),
    );
    (*p).sz = PGSIZE;

    // prepare for the very first "return" from kernel to user.
    (*(*p).trapframe).epc = 0; // user program counter
    (*(*p).trapframe).sp = PGSIZE; // user stack pointer

    ptr::copy_nonoverlapping(b"initcode\0".as_ptr(), (*p).name.as_mut_ptr(), 9);
    (*p).cwd = namei("/\0".as_ptr());

    (*p).state = ProcState::RUNNABLE;
    (*p).lock.release();
}

/// Grow or shrink user memory by n bytes.
/// Return 0 on success, -1 on failure.
pub unsafe fn growproc(n: i32) -> i32 {
    let p = myproc();
    let mut sz = (*p).sz;

    if n > 0 {
        let new_sz = uvmalloc(
            (*p).pagetable as *mut usize,
            sz,
            sz + n as u64,
            PTE_W.try_into().unwrap(),
        );
        if new_sz == 0 {
            return -1;
        }
        sz = new_sz;
    } else if n < 0 {
        sz = uvmdealloc((*p).pagetable as *mut usize, sz, sz + n as u64);
    }

    (*p).sz = sz;
    0
}

/// Create a new process, copying the parent.
/// Sets up child kernel stack to return as if from fork() system call.
pub unsafe fn fork() -> i32 {
    let p = myproc();

    // Allocate process.
    let np = allocproc();
    if np.is_null() {
        return -1;
    }

    // Copy user memory from parent to child.
    if uvmcopy(
        (*p).pagetable as *mut usize,
        (*np).pagetable as *mut usize,
        (*p).sz,
    ) < 0
    {
        freeproc(np);
        (*np).lock.release();
        return -1;
    }

    (*np).sz = (*p).sz;

    // copy saved user registers.
    *(*np).trapframe = *(*p).trapframe;

    // Cause fork to return 0 in the child.
    (*(*np).trapframe).a0 = 0;

    // increment reference counts on open file descriptors.
    for i in 0..NOFILE {
        if !(*p).ofile[i].is_null() {
            (*np).ofile[i] = filedup((*p).ofile[i]);
        }
    }
    (*np).cwd = idup((*p).cwd);

    // copy process name
    ptr::copy_nonoverlapping((*p).name.as_ptr(), (*np).name.as_mut_ptr(), 16);

    let pid = (*np).pid;

    (*np).lock.release();

    WAIT_LOCK.acquire();
    (*np).parent = p;
    WAIT_LOCK.release();

    (*np).lock.acquire();
    (*np).state = ProcState::RUNNABLE;
    (*np).lock.release();

    pid
}

/// Pass p's abandoned children to init.
/// Caller must hold wait_lock.
unsafe fn reparent(p: *mut Proc) {
    for pp in PROCS.iter_mut() {
        if (*pp).parent == p {
            (*pp).parent = INITPROC;
            wakeup(INITPROC);
        }
    }
}

/// Exit the current process. Does not return.
/// An exited process remains in the zombie state
/// until its parent calls wait().
pub unsafe fn exit(status: i32) -> ! {
    let p = myproc();

    if p == INITPROC {
        panic!("init exiting");
    }

    // Close all open files.
    for fd in 0..NOFILE {
        if !(*p).ofile[fd].is_null() {
            fileclose((*p).ofile[fd]);
            (*p).ofile[fd] = ptr::null_mut();
        }
    }

    begin_op();
    iput((*p).cwd);
    end_op();
    (*p).cwd = ptr::null_mut();

    WAIT_LOCK.acquire();

    // Give any children to init.
    reparent(p);

    // Parent might be sleeping in wait().
    wakeup((*p).parent);

    (*p).lock.acquire();

    (*p).xstate = status;
    (*p).state = ProcState::ZOMBIE;

    WAIT_LOCK.release();

    // Jump into the scheduler, never to return.
    sched();
    panic!("zombie exit");
}

/// Wait for a child process to exit and return its pid.
/// Return -1 if this process has no children.
pub unsafe fn wait(addr: u64) -> i32 {
    let p = myproc();

    WAIT_LOCK.acquire();

    loop {
        // Scan through table looking for exited children.
        let mut havekids = false;

        for pp in PROCS.iter_mut() {
            if (*pp).parent == p {
                // make sure the child isn't still in exit() or swtch().
                (*pp).lock.acquire();

                havekids = true;
                if (*pp).state == ProcState::ZOMBIE {
                    // Found one.
                    let pid = (*pp).pid;
                    if addr != 0
                        && copyout(
                            (*p).pagetable,
                            addr,
                            &(*pp).xstate as *const i32 as *const u8,
                            core::mem::size_of::<i32>() as u64,
                        ) < 0
                    {
                        (*pp).lock.release();
                        WAIT_LOCK.release();
                        return -1;
                    }

                    freeproc(pp);
                    (*pp).lock.release();
                    WAIT_LOCK.release();
                    return pid;
                }
                (*pp).lock.release();
            }
        }

        // No point waiting if we don't have any children.
        if !havekids || killed(p) {
            WAIT_LOCK.release();
            return -1;
        }

        // Wait for a child to exit.
        sleep(p, &WAIT_LOCK); //DOC: wait-sleep
    }
}

/// Per-CPU process scheduler.
/// Each CPU calls scheduler() after setting itself up.
/// Scheduler never returns. It loops, doing:
///  - choose a process to run.
///  - swtch to start running that process.
///  - eventually that process transfers control
///    via swtch back to the scheduler.
pub unsafe fn scheduler() -> ! {
    let c = mycpu();
    (*c).proc = ptr::null_mut();

    loop {
        // The most recent process to run may have had interrupts
        // turned off; enable them to avoid a deadlock if all
        // processes are waiting.
        intr_on();

        let mut found = false;
        for p in PROCS.iter_mut() {
            (*p).lock.acquire();
            if (*p).state == ProcState::RUNNABLE {
                // Switch to chosen process. It is the process's job
                // to release its lock and then reacquire it
                // before jumping back to us.
                (*p).state = ProcState::RUNNABLE;
                (*c).proc = p;
                swtch(&(*c).context, &(*p).context);

                // Process is done running for now.
                // It should have changed its p->state before coming back.
                (*c).proc = ptr::null_mut();
                found = true;
            }
            (*p).lock.release();
        }

        if !found {
            // nothing to run; stop running on this core until an interrupt.
            intr_on();
            core::arch::asm!("wfi");
        }
    }
}

/// Switch to scheduler. Must hold only p->lock
/// and have changed proc->state. Saves and restores
/// intena because intena is a property of this
/// kernel thread, not this CPU. It should
/// be proc->intena and proc->noff, but that would
/// break in the few places where a lock is held but
/// there's no process.
pub unsafe fn sched() {
    let p = myproc();
    let c = mycpu();

    if !(*p).lock.holding() {
        panic!("sched p->lock");
    }
    if (*c).noff != 1 {
        panic!("sched locks");
    }
    if (*p).state == ProcState::RUNNABLE {
        panic!("sched running");
    }
    if intr_get() {
        panic!("sched interruptible");
    }

    let intena = (*c).intena;
    swtch(&(*p).context, &(*c).context);
    (*c).intena = intena;
}

/// Give up the CPU for one scheduling round.
pub unsafe fn yield_proc() {
    let p = myproc();
    (*p).lock.acquire();
    (*p).state = ProcState::RUNNABLE;
    sched();
    (*p).lock.release();
}

/// A fork child's very first scheduling by scheduler()
/// will swtch to forkret.
pub unsafe fn forkret() {
    static FIRST: AtomicBool = AtomicBool::new(true);

    // Still holding p->lock from scheduler.
    (*myproc()).lock.release();

    // Use atomic operation for first time check
    if FIRST.swap(false, Ordering::SeqCst) {
        // File system initialization must be run in the context of a
        // regular process (e.g., because it calls sleep), and thus cannot
        // be run from main().
        fsinit(ROOTDEV);

        // Ensure other cores see first=false.
        // The `swap` operation above already ensures synchronization,
        // so no additional fence is needed here.
    }

    usertrapret();
}

/// Atomically release lock and sleep on chan.
/// Reacquires lock when awakened.
pub unsafe fn sleep(chan: *mut u8, lk: &mut SpinLock) {
    let p = myproc();

    // Must acquire p->lock in order to
    // change p->state and then call sched.
    // Once we hold p->lock, we can be
    // guaranteed that we won't miss any wakeup
    // (wakeup locks p->lock),
    // so it's okay to release lk.
    (*p).lock.acquire(); //DOC: sleeplock1
    lk.release();

    // Go to sleep.
    (*p).chan = chan;
    (*p).state = ProcState::SLEEPING;

    sched();

    // Tidy up.
    (*p).chan = ptr::null_mut();

    // Reacquire original lock.
    (*p).lock.release();
    lk.acquire();
}

/// Wake up all processes sleeping on chan.
/// Must be called without any p->lock.
pub unsafe fn wakeup(chan: *mut u8) {
    for p in PROCS.iter_mut() {
        if p as *mut Proc != myproc() {
            (*p).lock.acquire();
            if (*p).state == ProcState::SLEEPING && (*p).chan == chan {
                (*p).state = ProcState::RUNNABLE;
            }
            (*p).lock.release();
        }
    }
}

/// Kill the process with the given pid.
/// The victim won't exit until it tries to return
/// to user space (see usertrap() in trap.c).
pub unsafe fn kill(pid: i32) -> i32 {
    for p in PROCS.iter_mut() {
        (*p).lock.acquire();
        if (*p).pid == pid {
            (*p).killed = 1;
            if (*p).state == ProcState::SLEEPING {
                // Wake process from sleep().
                (*p).state = ProcState::RUNNABLE;
            }
            (*p).lock.release();
            return 0;
        }
        (*p).lock.release();
    }
    -1
}

pub unsafe fn setkilled(p: *mut Proc) {
    (*p).lock.acquire();
    (*p).killed = 1;
    (*p).lock.release();
}

pub unsafe fn killed(p: *mut Proc) -> uint {
    (*p).lock.acquire();
    let k = (*p).killed;
    (*p).lock.release();
    k.try_into().unwrap()
}

/// Copy to either a user address, or kernel address,
/// depending on usr_dst.
/// Returns 0 on success, -1 on error.
pub unsafe fn either_copyout(user_dst: bool, dst: u64, src: *const u8, len: u64) -> i32 {
    let p = myproc();
    if user_dst {
        copyout((*p).pagetable as *mut usize, dst, src, len)
    } else {
        ptr::copy_nonoverlapping(src, dst as *mut u8, len as usize);
        0
    }
}

/// Copy from either a user address, or kernel address,
/// depending on usr_src.
/// Returns 0 on success, -1 on error.
pub unsafe fn either_copyin(dst: *mut u8, user_src: bool, src: u64, len: u64) -> i32 {
    let p = myproc();
    if user_src {
        copyin((*p).pagetable, dst, src, len)
    } else {
        ptr::copy_nonoverlapping(src as *const u8, dst, len as usize);
        0
    }
}

/// Print a process listing to console. For debugging.
/// Runs when user types ^P on console.
/// No lock to avoid wedging a stuck machine further.
pub unsafe fn procdump() {
    println!("\n");
    for p in PROCS.iter() {
        if (*p).state == ProcState::UNUSED {
            continue;
        }

        let state = match (*p).state {
            ProcState::UNUSED => "unused",
            ProcState::USED => "used",
            ProcState::SLEEPING => "sleep ",
            ProcState::RUNNABLE => "runble",
            ProcState::RUNNING => "run   ",
            ProcState::ZOMBIE => "zombie",
        };

        println!(
            "{} {} {}",
            (*p).pid,
            state,
            core::str::from_utf8_unchecked(&(*p).name)
        );
    }
}
