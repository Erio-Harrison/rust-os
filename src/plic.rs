use crate::{memlayout::*, proc::cpuid};
use core::ptr;

/// Initialize the PLIC
pub unsafe fn plicinit() {
    // Set desired IRQ priorities non-zero (otherwise disabled).
    ptr::write_volatile((PLIC + UART0_IRQ * 4) as *mut u32, 1);
    ptr::write_volatile((PLIC + VIRTIO0_IRQ * 4) as *mut u32, 1);
}

/// Initialize PLIC for this hart
pub unsafe fn plicinithart() {
    let hart = cpuid();

    // Set enable bits for this hart's S-mode
    // for the uart and virtio disk.
    ptr::write_volatile(
        PLIC_SENABLE(hart.try_into().unwrap()) as *mut u32,
        (1 << UART0_IRQ) | (1 << VIRTIO0_IRQ)
    );

    // Set this hart's S-mode priority threshold to 0.
    ptr::write_volatile(PLIC_SPRIORITY(hart.try_into().unwrap()) as *mut u32, 0);
}

/// Ask the PLIC what interrupt we should serve.
/// Returns the IRQ number.
pub unsafe fn plic_claim() -> i32 {
    let hart = cpuid();
    ptr::read_volatile(PLIC_SCLAIM(hart.try_into().unwrap()) as *mut u32) as i32
}

/// Tell the PLIC we've served this IRQ.
pub unsafe fn plic_complete(irq: i32) {
    let hart = cpuid();
    ptr::write_volatile(PLIC_SCLAIM(hart.try_into().unwrap()) as *mut u32, irq as u32);
}