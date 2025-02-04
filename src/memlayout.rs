// memlayout.rs

//! Physical memory layout
//!
//! qemu -machine virt is set up like this,
//! based on qemu's hw/riscv/virt.c:
//!
//! 00001000 -- boot ROM, provided by qemu
//! 02000000 -- CLINT
//! 0C000000 -- PLIC
//! 10000000 -- uart0
//! 10001000 -- virtio disk
//! 80000000 -- boot ROM jumps here in machine mode
//!             -kernel loads the kernel here
//! unused RAM after 80000000.

use crate::riscv_local::MAXVA;
pub use crate::riscv_local::PGSIZE;

// qemu puts UART registers here in physical memory.
pub const UART0: u64 = 0x1000_0000;
pub const UART0_IRQ: u64 = 10;

// virtio mmio interface
pub const VIRTIO0: u64 = 0x1000_1000;
pub const VIRTIO0_IRQ: u64 = 1;

// qemu puts platform-level interrupt controller (PLIC) here.
pub const PLIC: u64 = 0x0c00_0000;
pub const PLIC_PRIORITY: u64 = PLIC + 0x0;
pub const PLIC_PENDING: u64 = PLIC + 0x1000;

/// PLIC supervisor enable registers for hart n
#[inline]
pub const fn PLIC_SENABLE(hart: u64) -> u64 {
    PLIC + 0x2080 + hart * 0x100
}

/// PLIC supervisor priority registers for hart n
#[inline]
pub const fn PLIC_SPRIORITY(hart: u64) -> u64 {
    PLIC + 0x201000 + hart * 0x2000
}

/// PLIC supervisor claim registers for hart n
#[inline]
pub const fn PLIC_SCLAIM(hart: u64) -> u64 {
    PLIC + 0x201004 + hart * 0x2000
}

// The kernel expects there to be RAM
// for use by the kernel and user pages
// from physical address 0x80000000 to PHYSTOP.
pub const KERNBASE: u64 = 0x8000_0000;
pub const PHYSTOP: u64 = KERNBASE + 128 * 1024 * 1024;

// Map the trampoline page to the highest address,
// in both user and kernel space.
pub const TRAMPOLINE: u64 = MAXVA - PGSIZE;

/// Map kernel stacks beneath the trampoline,
/// each surrounded by invalid guard pages.
#[inline]
pub const fn KSTACK(p: usize) -> u64 {
    TRAMPOLINE - ((p as u64) + 1) * 2 * PGSIZE
}

// User memory layout.
// Address zero first:
//   text
//   original data and bss
//   fixed-size stack
//   expandable heap
//   ...
//   TRAPFRAME (p->trapframe, used by the trampoline)
//   TRAMPOLINE (the same page as in the kernel)
pub const TRAPFRAME: u64 = TRAMPOLINE - PGSIZE;
