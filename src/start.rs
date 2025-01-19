use crate::{riscv::*, uart::{self, debug_print}};

extern "C" {
    fn rust_main() -> !;
}

#[no_mangle]
#[link_section = ".text"]
pub unsafe extern "C" fn start() -> ! {
    // 先不做UART初始化，专注于跳转逻辑
    
    // Set M Previous Privilege mode to Supervisor
    let mut x = r_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    w_mstatus(x);

    // Set M Exception Program Counter to main
    w_mepc(rust_main as u64);

    // Disable paging
    w_satp(0);

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