// src/trap/mod.rs

use core::arch::asm;

use crate::println;

// Define CSR register operation macro
macro_rules! read_csr {
    ($reg:literal) => {
        {
            let value: usize;
            unsafe {
                asm!(concat!("csrr {}, ", $reg), out(reg) value);
            }
            value
        }
    };
}

macro_rules! write_csr {
    ($reg:literal, $value:expr) => {
        unsafe {
            asm!(concat!("csrw ", $reg, ", {}"), in(reg) $value);
        }
    };
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
        }
    }
}

// Initialize trap processing
pub fn init() {
    extern "C" {
        fn __alltraps();
    }

    println!("Setting up trap handler at {:#x}", __alltraps as usize);

    // Set stvec to use Direct mode
    write_csr!("stvec", __alltraps as usize);

    // Setting the privilege level
    unsafe {
        // Make sure you are in S mode
        asm!("csrw sstatus, {}", in(reg) 0x100);
    }

    println!("Current sstatus: {:#x}", read_csr!("sstatus"));
    println!("Current sie: {:#x}", read_csr!("sie"));
    println!("Trap handler setup complete!");
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

#[no_mangle]
pub extern "C" fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = read_csr!("scause");
    let stval = read_csr!("stval");

    println!("Trap handler entered!");
    println!(
        "scause: {:#x}, stval: {:#x}, sepc: {:#x}",
        scause, stval, cx.sepc
    );

    let (trap_type, code) = parse_scause(scause);
    println!("Trap type: {:?}, code: {}", trap_type, code);

    match trap_type {
        TrapType::Exception => {
            match code {
                3 => {
                    println!("Breakpoint at 0x{:x}", cx.sepc);
                    cx.sepc += 2; // ebreak is a 2-byte instruction
                    return cx;
                }
                7 => {
                    // Store/AMO access fault
                    println!("Store access fault at address 0x{:x}", stval);
                    panic!("Store access fault!");
                }
                5 => {
                    // Load access fault
                    println!("Load access fault at address 0x{:x}", stval);
                    panic!("Load access fault!");
                }
                _ => {
                    println!("Unknown exception code: {}", code);
                    panic!("Unhandled exception! code={}", code);
                }
            }
        }
        TrapType::Interrupt => {
            println!("Got interrupt, code: {}", code);
            panic!("Unhandled interrupt!");
        }
    }
}
