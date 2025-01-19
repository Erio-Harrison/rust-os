use crate::types::uint64;

// Machine Status Register (mstatus) related constants
pub const MSTATUS_MPP_MASK: uint64 = 3 << 11; // previous mode
pub const MSTATUS_MPP_M: uint64 = 3 << 11; // machine mode
pub const MSTATUS_MPP_S: uint64 = 1 << 11; // supervisor mode
pub const MSTATUS_MPP_U: uint64 = 0 << 11; // user mode
pub const MSTATUS_MIE: uint64 = 1 << 3; // machine-mode interrupt enable

/// Returns the hardware thread (hart) ID
#[inline]
pub unsafe fn r_mhartid() -> uint64 {
    let x: uint64;
    core::arch::asm!("csrr {}, mhartid", out(reg) x);
    x
}

/// Read machine status register (mstatus)
#[inline(never)]
#[no_mangle]
pub unsafe fn r_mstatus() -> uint64 {
    let x: uint64;
    core::arch::asm!("csrr {}, mstatus", out(reg) x);
    x
}

/// Write machine status register (mstatus)
#[inline]
pub unsafe fn w_mstatus(x: uint64) {
    core::arch::asm!("csrw mstatus, {}", in(reg) x);
}

/// Write machine exception program counter (mepc)
/// mepc holds the instruction address to which a return from exception will go
#[inline]
pub unsafe fn w_mepc(x: uint64) {
    core::arch::asm!("csrw mepc, {}", in(reg) x);
}

// Supervisor Status Register, sstatus
pub const SSTATUS_SPP: u64 = 1 << 8; // Previous mode, 1=Supervisor, 0=User
pub const SSTATUS_SPIE: u64 = 1 << 5; // Supervisor Previous Interrupt Enable
pub const SSTATUS_UPIE: u64 = 1 << 4; // User Previous Interrupt Enable
pub const SSTATUS_SIE: u64 = 1 << 1; // Supervisor Interrupt Enable
pub const SSTATUS_UIE: u64 = 1 << 0; // User Interrupt Enable

#[inline]
pub unsafe fn r_sstatus() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, sstatus", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_sstatus(x: u64) {
    core::arch::asm!("csrw sstatus, {}", in(reg) x);
}

// Supervisor Interrupt Pending
#[inline]
pub unsafe fn r_sip() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, sip", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_sip(x: u64) {
    core::arch::asm!("csrw sip, {}", in(reg) x);
}

// Supervisor Interrupt Enable
pub const SIE_SEIE: u64 = 1 << 9; // external
pub const SIE_STIE: u64 = 1 << 5; // timer
pub const SIE_SSIE: u64 = 1 << 1; // software

#[inline]
pub unsafe fn r_sie() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, sie", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_sie(x: u64) {
    core::arch::asm!("csrw sie, {}", in(reg) x);
}

// Machine-mode Interrupt Enable
pub const MIE_STIE: u64 = 1 << 5; // supervisor timer

#[inline]
pub unsafe fn r_mie() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, mie", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_mie(x: u64) {
    core::arch::asm!("csrw mie, {}", in(reg) x);
}

// supervisor exception program counter, holds the
// instruction address to which a return from
// exception will go.
#[inline]
pub unsafe fn w_sepc(x: u64) {
    core::arch::asm!("csrw sepc, {}", in(reg) x);
}

#[inline]
pub unsafe fn r_sepc() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, sepc", out(reg) x);
    x
}

// Machine Exception Delegation
#[inline]
pub unsafe fn r_medeleg() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, medeleg", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_medeleg(x: u64) {
    core::arch::asm!("csrw medeleg, {}", in(reg) x);
}

// Machine Interrupt Delegation
#[inline]
pub unsafe fn r_mideleg() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, mideleg", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_mideleg(x: u64) {
    core::arch::asm!("csrw mideleg, {}", in(reg) x);
}

// Supervisor Trap-Vector Base Address
// low two bits are mode.
#[inline]
pub unsafe fn w_stvec(x: u64) {
    core::arch::asm!("csrw stvec, {}", in(reg) x);
}

#[inline]
pub unsafe fn r_stvec() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, stvec", out(reg) x);
    x
}

// Supervisor Timer Comparison Register
#[inline]
pub unsafe fn r_stimecmp() -> u64 {
    let x: u64;
    // Using raw CSR number because stimecmp is not a standard name
    core::arch::asm!("csrr {}, 0x14d", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_stimecmp(x: u64) {
    // Using raw CSR number because stimecmp is not a standard name
    core::arch::asm!("csrw 0x14d, {}", in(reg) x);
}

// Machine Environment Configuration Register
#[inline]
pub unsafe fn r_menvcfg() -> u64 {
    let x: u64;
    // Using raw CSR number because menvcfg is not a standard name
    core::arch::asm!("csrr {}, 0x30a", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_menvcfg(x: u64) {
    // Using raw CSR number because menvcfg is not a standard name
    core::arch::asm!("csrw 0x30a, {}", in(reg) x);
}

// Physical Memory Protection
#[inline]
pub unsafe fn w_pmpcfg0(x: u64) {
    core::arch::asm!("csrw pmpcfg0, {}", in(reg) x);
}

#[inline]
pub unsafe fn w_pmpaddr0(x: u64) {
    core::arch::asm!("csrw pmpaddr0, {}", in(reg) x);
}

// use riscv's sv39 page table scheme
pub const SATP_SV39: u64 = 8 << 60;

#[inline]
pub fn make_satp(pagetable: u64) -> u64 {
    SATP_SV39 | ((pagetable) >> 12)
}

// supervisor address translation and protection;
// holds the address of the page table.
#[inline]
pub unsafe fn w_satp(x: u64) {
    core::arch::asm!("csrw satp, {}", in(reg) x);
}

#[inline]
pub unsafe fn r_satp() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, satp", out(reg) x);
    x
}

// Supervisor Trap Cause
#[inline]
pub unsafe fn r_scause() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, scause", out(reg) x);
    x
}

// Supervisor Trap Value
#[inline]
pub unsafe fn r_stval() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, stval", out(reg) x);
    x
}

// Machine-mode Counter-Enable
#[inline]
pub unsafe fn w_mcounteren(x: u64) {
    core::arch::asm!("csrw mcounteren, {}", in(reg) x);
}

#[inline]
pub unsafe fn r_mcounteren() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, mcounteren", out(reg) x);
    x
}

// machine-mode cycle counter
#[inline]
pub unsafe fn r_time() -> u64 {
    let x: u64;
    core::arch::asm!("csrr {}, time", out(reg) x);
    x
}

// enable device interrupts
#[inline]
pub unsafe fn intr_on() {
    w_sstatus(r_sstatus() | SSTATUS_SIE);
}

// disable device interrupts
#[inline]
pub unsafe fn intr_off() {
    w_sstatus(r_sstatus() & !SSTATUS_SIE);
}

// are device interrupts enabled?
#[inline]
pub unsafe fn intr_get() -> bool {
    let x = r_sstatus();
    (x & SSTATUS_SIE) != 0
}

// read stack pointer
#[inline]
pub unsafe fn r_sp() -> u64 {
    let x: u64;
    core::arch::asm!("mv {}, sp", out(reg) x);
    x
}

// read and write tp, the thread pointer, which xv6 uses to hold
// this core's hartid (core number), the index into cpus[].
#[inline]
pub unsafe fn r_tp() -> u64 {
    let x: u64;
    core::arch::asm!("mv {}, tp", out(reg) x);
    x
}

#[inline]
pub unsafe fn w_tp(x: u64) {
    core::arch::asm!("mv tp, {}", in(reg) x);
}

#[inline]
pub unsafe fn r_ra() -> u64 {
    let x: u64;
    core::arch::asm!("mv {}, ra", out(reg) x);
    x
}

// flush the TLB.
#[inline]
pub unsafe fn sfence_vma() {
    // the zero, zero means flush all TLB entries.
    core::arch::asm!("sfence.vma zero, zero");
}

// Page table related types
pub type PTE = u64;
pub type PageTable = *mut u64; // 512 PTEs

// Page size constants
pub const PGSIZE: u64 = 4096; // bytes per page
pub const PGSHIFT: u64 = 12; // bits of offset within a page

// Page table entry flags
pub const PTE_V: u64 = 1 << 0; // valid
pub const PTE_R: u64 = 1 << 1; // readable
pub const PTE_W: u64 = 1 << 2; // writable
pub const PTE_X: u64 = 1 << 3; // executable
pub const PTE_U: u64 = 1 << 4; // user can access

// Page table index extraction constants
pub const PXMASK: u64 = 0x1FF; // 9 bits

// Maximum virtual address
// MAXVA is actually one bit less than the max allowed by
// Sv39, to avoid having to sign-extend virtual addresses
// that have the high bit set.
pub const MAXVA: u64 = 1 << (9 + 9 + 9 + 12 - 1);

// Page table helper functions
#[inline]
pub fn pgroundup(sz: u64) -> u64 {
    (sz + PGSIZE as u64 - 1) & !(PGSIZE as u64 - 1)
}

#[inline]
pub fn pgrounddown(addr: u64) -> u64 {
    addr & !(PGSIZE as u64 - 1)
}

// shift a physical address to the right place for a PTE.
#[inline]
pub fn pa2pte(pa: u64) -> u64 {
    (pa >> 12) << 10
}

#[inline]
pub fn pte2pa(pte: u64) -> u64 {
    (pte >> 10) << 12
}

#[inline]
pub fn pte_flags(pte: u64) -> u64 {
    pte & 0x3FF
}
