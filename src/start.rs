use crate::{riscv::*, uart::{self, debug_print}};

extern "C" {
    fn rust_main() -> !;
}

pub unsafe extern "C" fn start() -> ! {
    // 初始化UART
    uart::uartinit();
    
    debug_print("xv6 kernel is booting\n");

    // Set M Previous Privilege mode to Supervisor
    let mut x = r_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    w_mstatus(x);

    // Set M Exception Program Counter to main
    w_mepc(rust_main as u64);

    // Disable paging
    w_satp(0);

    // Delegate interrupts and exceptions
    w_medeleg(0xffff);
    w_mideleg(0xffff);
    w_sie(r_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);

    // Configure Physical Memory Protection
    w_pmpaddr0(0x3fffffffffffff as u64);
    w_pmpcfg0(0xf);

    // Store hartid in tp for cpuid()
    let id = r_mhartid();
    w_tp(id);
  
    // Initialize timer interrupts
    timerinit();

    // Switch to supervisor mode and jump to main
    unsafe {
        core::arch::asm!("mret", options(noreturn));
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