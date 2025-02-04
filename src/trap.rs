// src/trap.rs
use crate::{memlayout::{TRAMPOLINE, UART0_IRQ, VIRTIO0_IRQ}, 
plic::{plic_claim, plic_complete}, println, 
proc::{cpuid, exit, myproc, wakeup, yield_proc},
 riscv_local::{intr_get, intr_off, intr_on, make_satp, r_satp, r_scause, r_sepc, r_sstatus, r_stval, r_time, r_tp, w_sepc, w_sstatus, w_stimecmp, w_stvec, PGSIZE, SSTATUS_SPIE, SSTATUS_SPP}, spinlock::SpinLock, syscall::syscall,
  types::uint64};

pub static mut TICKSLOCK: SpinLock = SpinLock::new("time\0".as_bytes().as_ptr());
pub static mut TICKS: usize = 0;

extern "C" {
    fn trampoline();
    fn uservec();
    fn userret();
    fn kernelvec();
}

// Interrupt context structure
#[repr(C)]
#[derive(Debug)]
pub struct TrapContext {
    // General registers
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    // CSR Register
    pub sstatus: usize,
    pub sepc: usize,

    // Kernel registers
    pub kernel_satp: usize, // kernel page table
    pub kernel_sp: usize, // kernel stack pointer
    pub kernel_trap: usize, // trap processing function address
    pub kernel_hartid: usize, // hart id
}

impl TrapContext {
    pub fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            gp: 0,
            tp: 0,
            t0: 0,
            t1: 0,
            t2: 0,
            s0: 0,
            s1: 0,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            a5: 0,
            a6: 0,
            a7: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
            t3: 0,
            t4: 0,
            t5: 0,
            t6: 0,
            sstatus: 0,
            sepc: 0,
            kernel_satp: 0,
            kernel_sp: 0,
            kernel_trap: 0,
            kernel_hartid: 0,
        }
    }
    
}

/// Initialize trap module
pub unsafe fn trapinit() {
    TICKSLOCK.initlock("time\0".as_bytes().as_ptr());
}

/// Set up to take exceptions and traps while in the kernel
pub unsafe fn trapinithart() {
    w_stvec(kernelvec as u64);
}

// Trap Type Enumeration
#[derive(Debug)]
pub enum TrapType {
    Interrupt = 0,
    Exception = 1,
}

// Specific exception type
#[derive(Debug)]
pub enum ExceptionType {
    InstructionMisaligned = 0,
    InstructionFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadMisaligned = 4,
    LoadFault = 5,
    StoreMisaligned = 6,
    StoreFault = 7,
    UserEnvCall = 8,
    SupervisorEnvCall = 9,
    MachineEnvCall = 11,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    Unknown,
}

// Parse the exception type from the cause register
fn parse_scause(scause: usize) -> (TrapType, u64) {
    let trap_type = if scause >> 63 == 0 {
        TrapType::Exception
    } else {
        TrapType::Interrupt
    };

    // Take the lower 12 bits as the exception code
    let code = (scause & 0xfff) as u64;
    (trap_type, code)
}

/// Handle an interrupt, exception, or system call from user space.
/// Called from trampoline.S
#[no_mangle]
pub unsafe fn usertrap() {
    let mut which_dev = 0;

    if (r_sstatus() & SSTATUS_SPP) != 0 {
        panic!("usertrap: not from user mode");
    }

    // Send interrupts and exceptions to kerneltrap(),
    // since we're now in the kernel.
    w_stvec(kernelvec as u64);

    let p = myproc();

    // Save user program counter.
    (*(*p).trapframe).epc = r_sepc();

    let scause = r_scause();
    if scause == 8 {
        // System call
        if (*p).killed != 0 {
            exit(-1);
        }

        // sepc points to the ecall instruction,
        // but we want to return to the next instruction.
        (*(*p).trapframe).epc += 4;

        // An interrupt will change sepc, scause, and sstatus,
        // so enable only now that we're done with those registers.
        intr_on();

        syscall();
    } else {
        which_dev = devintr();
        if which_dev == 0 {
            println!(
                "usertrap(): unexpected scause {:#x} pid={}",
                scause,
                (*p).pid
            );
            println!(
                "            sepc={:#x} stval={:#x}",
                r_sepc(), 
                r_stval()
            );
            (*p).killed = 1;
        }
    }

    if (*p).killed != 0 {
        exit(-1);
    }

    // Give up the CPU if this is a timer interrupt.
    if which_dev == 2 {
        yield_proc();
    }

    usertrapret();
}
/// Return to user space
pub unsafe fn usertrapret() {
    let p = myproc();

    // We're about to switch the destination of traps from
    // kerneltrap() to usertrap(), so turn off interrupts until
    // we're back in user space, where usertrap() is correct.
    intr_off();

    // Send syscalls, interrupts, and exceptions to uservec in trampoline.S
    let trampoline_uservec = TRAMPOLINE + (uservec as u64 - trampoline as u64);
    w_stvec(trampoline_uservec);

    // Set up trapframe values that uservec will need when
    // the process next traps into the kernel.
    (*(*p).trapframe).kernel_satp = r_satp();          // kernel page table
    (*(*p).trapframe).kernel_sp = (*p).kstack + PGSIZE;        // process's kernel stack
    (*(*p).trapframe).kernel_trap = usertrap as u64;
    (*(*p).trapframe).kernel_hartid = r_tp();    // hartid for cpuid()

    // Set up the registers that trampoline.S's sret will use
    // to get to user space.
    
    // Set S Previous Privilege mode to User.
    let mut sstatus = r_sstatus();
    sstatus &= !SSTATUS_SPP;  // clear SPP to 0 for user mode
    sstatus |= SSTATUS_SPIE;  // enable interrupts in user mode
    w_sstatus(sstatus);

    // Set S Exception Program Counter to the saved user pc.
    w_sepc((*(*p).trapframe).epc);

    // Tell trampoline.S the user page table to switch to.
    let satp = make_satp(((*p).pagetable as usize).try_into().unwrap());

    // Jump to userret in trampoline.S at the top of memory, which
    // switches to the user page table, restores user registers,
    // and switches to user mode with sret.
    let trampoline_userret = TRAMPOLINE + (userret as u64 - trampoline as u64);
    let fn_: extern "C" fn(usize) = core::mem::transmute(trampoline_userret);
    fn_(satp.try_into().unwrap());
}

/// Interrupts and exceptions from kernel code go here via kernelvec,
/// on whatever the current kernel stack is.
#[no_mangle]
pub unsafe fn kerneltrap() {
    let mut which_dev = 0;
    let sepc = r_sepc();
    let sstatus = r_sstatus();
    let scause = r_scause();

    if (sstatus & SSTATUS_SPP) == 0 {
        panic!("kerneltrap: not from supervisor mode");
    }
    if intr_get() {
        panic!("kerneltrap: interrupts enabled");
    }

    which_dev = devintr();
    if which_dev == 0 {
        println!("scause={:#x}", scause);
        println!("sepc={:#x}", sepc);
        println!("stval={:#x}", r_stval());
        panic!("kerneltrap");
    }

    // Give up the CPU if this is a timer interrupt.
    if which_dev == 2 && !myproc().is_null() {
        yield_proc();
    }

    // The yield() may have caused some traps to occur,
    // so restore trap registers for use by kernelvec.S's sepc instruction.
    w_sepc(sepc);
    w_sstatus(sstatus);
}

unsafe fn clockintr() {
    if cpuid() == 0 {
        TICKSLOCK.acquire();
        TICKS += 1;
        wakeup(&TICKS as *const _ as *mut u8);
        TICKSLOCK.release();
    }

    // Ask for the next timer interrupt. This also clears
    // the interrupt request. 1000000 is about a tenth of a second.
    w_stimecmp(r_time() + 1000000);
}

use crate::virtio::DISK;

/// Check if it's an external interrupt or software interrupt,
/// and handle it.
/// Returns 2 if timer interrupt,
/// 1 if other device,
/// 0 if not recognized.
unsafe fn devintr() -> i32 {
    let scause =  r_scause();

    if scause == 0x8000000000000009 {
        // This is a supervisor external interrupt, via PLIC.

        // irq indicates which device interrupted.
        let irq = plic_claim();

        if irq == UART0_IRQ as i32 {
            //uartintr();
            println!("I am testing!")
        } else if irq == VIRTIO0_IRQ as i32 {
            DISK.virtio_disk_intr();
        } else if irq != 0 {
            println!("unexpected interrupt irq={}", irq);
        }

        // The PLIC allows each device to raise at most one
        // interrupt at a time; tell the PLIC the device is
        // now allowed to interrupt again.
        if irq != 0 {
            plic_complete(irq);
        }

        1
    } else if scause == 0x8000000000000005 {
        // Timer interrupt.
        clockintr();
        2
    } else {
        0
    }
}