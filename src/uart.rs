// src/uart.rs

//! UART Controller Driver
//! The UART controller of QEMU RISC-V Virt machine is NS16550A

use core::ptr::{read_volatile, write_volatile};

// Physical address of the UART for the QEMU virt machine
const UART_BASE_ADDR: usize = 0x1000_0000;

// UART register offsets
const RBR: usize = 0x0; // Receiver Buffer Register (read)
const THR: usize = 0x0; // Transmitter Holding Register (write)
const DLL: usize = 0x0; // Divisor Latch Low Byte
const DLM: usize = 0x1; // Divisor Latch High Byte
const IER: usize = 0x1; // Interrupt Enable Register
const FCR: usize = 0x2; // FIFO Control Register
const LCR: usize = 0x3; // Line Control Register
const LSR: usize = 0x5; // Line Status Register

// Line Status Register bits
const LSR_DR: u8 = 1 << 0; // Data Ready
const LSR_THRE: u8 = 1 << 5; // Transmitter Holding Register Empty

/// Initialize the UART
pub fn init() {
    unsafe {
        // Disable interrupts
        write_reg(IER, 0x00);

        // Set baud rate
        write_reg(LCR, 0x80); // Set DLAB bit to allow baud rate configuration
        write_reg(DLL, 0x03); // Set divisor to 3, baud rate to 38.4K
        write_reg(DLM, 0x00);

        // Configure transmission format: 8 data bits, 1 stop bit, no parity
        write_reg(LCR, 0x03);

        // Enable FIFO, clear FIFO
        write_reg(FCR, 0x07);

        // Enable interrupts
        write_reg(IER, 0x01);
    }
}

/// Write a byte
pub fn putchar(c: u8) {
    unsafe {
        // Wait until the Transmitter Holding Register is empty
        while (read_reg(LSR) & LSR_THRE) == 0 {}
        write_reg(THR, c);
    }
}

/// Read a byte
pub fn getchar() -> Option<u8> {
    unsafe {
        if (read_reg(LSR) & LSR_DR) == 0 {
            None
        } else {
            Some(read_reg(RBR))
        }
    }
}

/// Read from a register
unsafe fn read_reg(reg: usize) -> u8 {
    read_volatile((UART_BASE_ADDR + reg) as *const u8)
}

/// Write to a register
unsafe fn write_reg(reg: usize, val: u8) {
    write_volatile((UART_BASE_ADDR + reg) as *mut u8, val)
}
