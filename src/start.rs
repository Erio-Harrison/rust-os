use crate::{memlayout::*, param::NCPU, riscv::*};
use core::arch::asm;

extern "C" {
    fn main() -> !;
}

/// Machine mode startup code
#[no_mangle]
pub unsafe extern "C" fn start() -> ! {
    // Set M Previous Privilege mode to Supervisor, for mret
    let mut x = r_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    w_mstatus(x);

    // Set M Exception Program Counter to main, for mret
    w_mepc(main as u64);

    // Disable paging for now
    w_satp(0);

    // Delegate all interrupts and exceptions to supervisor mode
    w_medeleg(0xffff);
    w_mideleg(0xffff);
    w_sie(r_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);

    // Configure Physical Memory Protection
    w_pmpaddr0(0x3fffffffffffffffu64);
    w_pmpcfg0(0xf);

    // Ask for clock interrupts
    timerinit();

    // Keep each CPU's hartid in its tp register, for cpuid()
    let id = r_mhartid();
    w_tp(id);

    // Switch to supervisor mode and jump to main
    unsafe {
        asm!("mret", options(noreturn));
    }
}

/// Initialize timer interrupts
unsafe fn timerinit() {
    // Enable supervisor-mode timer interrupts
    w_mie(r_mie() | MIE_STIE);

    // Enable the sstc extension
    w_menvcfg(r_menvcfg() | (1u64 << 63));

    // Allow supervisor to use stimecmp and time
    w_mcounteren(r_mcounteren() | 2);

    // Set first timer interrupt
    w_stimecmp(r_time() + 1000000);
}
