use crate::riscv::*;

extern "C" {
    fn rust_main() -> !;
}

#[no_mangle]
#[link_section = ".text"]
pub unsafe extern "C" fn start() -> ! {
    let mut x = r_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    w_mstatus(x);

    // set M Exception Program Counter
    w_mepc(rust_main as u64);

    // Disable paging
    w_satp(0);


    w_medeleg(0xffff);
    w_mideleg(0xffff);
    w_sie(r_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);

    w_pmpaddr0(0x3fffffffffffff_u64);
    w_pmpcfg0(0xf);

    // Switch to supervisor mode and jump to main
    unsafe {
        core::arch::asm!("mret", options(noreturn));
    }
}