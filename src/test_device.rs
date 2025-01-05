// src/test_device.rs
use core::ptr::write_volatile;

use crate::println;

// SiFive test device MMIO address (based on QEMU virt machine memory map)
const SIFIVE_TEST_BASE: usize = 0x100000;

// Exit codes
const TEST_EXIT_PASS: u32 = 0x5555;
const TEST_EXIT_FAIL: u32 = 0x3333;

/// Initialize the test device - SiFive Test device does not require initialization
pub fn init() {
    // SiFive test device does not need initialization
}

/// Exit QEMU with a pass status
pub fn exit_pass() -> ! {
    unsafe {
        // Print confirmation message
        println!("Exiting QEMU with success status...");
        // Write exit code
        write_reg(0, TEST_EXIT_PASS);
        // Theoretically, this point should never be reached as QEMU should have exited
        loop {}
    }
}

/// Exit QEMU with a fail status
pub fn exit_fail() -> ! {
    unsafe {
        // Print confirmation message
        println!("Exiting QEMU with failure status...");
        // Write exit code
        write_reg(0, TEST_EXIT_FAIL);
        // Theoretically, this point should never be reached as QEMU should have exited
        loop {}
    }
}

// Write to a device register
unsafe fn write_reg(offset: usize, val: u32) {
    write_volatile((SIFIVE_TEST_BASE + offset) as *mut u32, val)
}