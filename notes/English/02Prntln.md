Let's implement basic printing functionality. Before we formally begin, we need to introduce RISC-V privilege levels.

RISC-V defines three main privilege levels (X86 has four privilege levels):

1. Machine Mode (M Mode):
- Highest privilege level, direct hardware access
- Typically used by firmware (like OpenSBI) or the lowest layer of the operating system kernel

2. Supervisor Mode (S Mode):
- Used for operating system kernels
- Can access some hardware resources but needs to go through M mode interface (like SBI) to access lower-level hardware

3. User Mode (U Mode):
- Used for applications
- Cannot directly access hardware, must use system calls (S mode) or SBI (M mode) to access hardware

We used two methods for implementing printing: `UART` and `SBI`. The `SBI` is currently commented out:

```rust
use core::fmt::{self, Write};
//use crate::sbi::console_putchar;
use crate::uart::putchar;

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            //console_putchar(c as usize);
            putchar(c as u8);
        }
        Ok(())
    }
}
```

`UART` is an S-level operation, implemented through direct reading and writing to memory-mapped registers. These registers' addresses are physical addresses (such as UART_BASE_ADDR = 0x1000_0000, which is the UART physical address of the QEMU virtual machine), and the operating system kernel (running in S mode) can directly access these addresses.

The `read_reg` and `write_reg` functions in `uart.rs` directly access UART registers through physical addresses:

```rust
/// Read from a register
unsafe fn read_reg(reg: usize) -> u8 {
    read_volatile((UART_BASE_ADDR + reg) as *const u8)
}

/// Write to a register
unsafe fn write_reg(reg: usize, val: u8) {
    write_volatile((UART_BASE_ADDR + reg) as *mut u8, val)
}
```

These two functions run in S mode and complete memory read/write operations through a memory-mapped address called `MMIO`. You can see this in the following diagram:

```
CPU Address Space
+------------------+ 0xFFFFFFFF
|      ...        |
+------------------+
| Peripheral Regs  | <- MMIO region (e.g., UART at 0x10000000)
+------------------+
|     Memory      |
+------------------+ 0x00000000
```

The CPU has a unified address bus, and the address decoder determines whether the accessed address belongs to memory or peripherals. If it's a peripheral address, the signal is routed to the corresponding peripheral controller, which handles the read/write request. Taking `write_volatile(0x10000000, 'A')` as an example, when the CPU executes this command:
1. The CPU first puts address 0x10000000 on the address bus
2. The address decoder recognizes this is in the UART address range
3. The write signal and data 'A' are routed to the UART controller
4. The UART controller receives the signal and puts character 'A' in the transmit buffer

Based on the QEMU-simulated NS16550A UART device register layout, we can define the following constants:

```rust
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
```

Through base address + offset (for example, the address to access the LCR register is `UART_BASE_ADDR + LCR`, i.e., `0x1000_0000 + 0x3 = 0x1000_0003`), we can get the locations of these registers. Note that they are all hardcoded because these offsets are hardware characteristics of the NS16550A UART, determined by the chip's register layout.

---

| Offset | Register Name | Access | Description |
|--------|--------------|---------|-------------|
| `0x0` | **RBR (Receiver Buffer Register)** | Read | Used to read received data. |
| `0x0` | **THR (Transmitter Holding Register)** | Write | Used to write data for transmission. |
| `0x0` | **DLL (Divisor Latch Low)** | Read/Write | Baud rate divisor latch low byte (accessed when DLAB=1). |
| `0x1` | **DLM (Divisor Latch High)** | Read/Write | Baud rate divisor latch high byte (accessed when DLAB=1). |
| `0x1` | **IER (Interrupt Enable Register)** | Read/Write | Controls interrupts. |
| `0x2` | **FCR (FIFO Control Register)** | Write | Controls FIFO buffer. |
| `0x3` | **LCR (Line Control Register)** | Read/Write | Configures data format. |
| `0x5` | **LSR (Line Status Register)** | Read | Checks UART status. |

---

Before formally starting data transmission, we need to initialize the UART by setting the relevant register parameters:

```rust
/// Initialize the UART
pub fn init() {
    unsafe {
        // Disable interrupts
        write_reg(IER, 0x00);

        // Set baud rate
        write_reg(LCR, 0x80); // Set DLAB bit to allow baud rate configuration
        write_reg(DLL, 0x03); // Set divisor to 3, baud rate to 38.4K
        write_reg(DLM, 0x00); // The high byte of the divisor is 0

        // Configure transmission format: 8 data bits, 1 stop bit, no parity
        write_reg(LCR, 0x03);

        // Enable FIFO, clear FIFO
        write_reg(FCR, 0x07);

        // Enable interrupts
        write_reg(IER, 0x01);
    }
}
```

---

### 1. **Disable All Interrupts**
```rust
write_reg(IER, 0x00);
```
- **IER (Interrupt Enable Register)**: Controls UART interrupts
- **`0x00`**: Sets all bits in IER register to 0, disabling all interrupts
- **Why disable interrupts?**
  - Prevents unnecessary interrupts during UART configuration
  - Ensures atomicity of the initialization process

---

### 2. **Set Baud Rate**
```rust
write_reg(LCR, 0x80);
write_reg(DLL, 0x03);
write_reg(DLM, 0x00);
```
- **LCR (Line Control Register)**: Configures data format and access to baud rate divisor registers
- **DLAB (Divisor Latch Access Bit)**: Bit 7 of LCR register, controls access to baud rate divisor registers
  - **`0x80`**: Sets DLAB bit to 1, enabling access to baud rate divisor registers (DLL and DLM)
- **DLL (Divisor Latch Low)** and **DLM (Divisor Latch High)**: Set baud rate
  - Baud rate formula: `Baud Rate = Clock Frequency / (16 * Divisor)`
  - **`DLL = 0x03` and `DLM = 0x00`**: Sets divisor to 3, assuming clock frequency of 1.8432 MHz, resulting in baud rate of `1.8432 MHz / (16 * 3) = 38,400 bps`
- **Why set baud rate?**
  - Baud rate determines data transmission speed. Sender and receiver must use the same baud rate for correct communication

---

### 3. **Configure Transmission Format**
```rust
write_reg(LCR, 0x03);
```
- **LCR (Line Control Register)**: Configures data format
- **`0x03`**:
  - Bits 0-1: Set 8 data bits
  - Bit 2: Set 1 stop bit
  - Bit 3: Disable parity
- **Why configure transmission format?**
  - Data format (data bits, stop bits, parity) must match between communicating parties for correct data transmission

---

### 4. **Enable and Clear FIFO**
```rust
write_reg(FCR, 0x07);
```
- **FCR (FIFO Control Register)**: Controls FIFO buffer behavior
- **`0x07`**:
  - Bit 0: Enable FIFO
  - Bits 1-2: Clear receive and transmit FIFOs
  - Bit 3: Reserved bit, typically set to 0
- **Why enable and clear FIFO?**
  - FIFO (First In First Out) buffers temporarily store received and transmitted data, improving data transfer efficiency
  - Clearing FIFO ensures no old data remains in buffers during initialization

---

### 5. **Enable Receive Interrupt**
```rust
write_reg(IER, 0x01);
```
- **IER (Interrupt Enable Register)**: Controls UART interrupts
- **`0x01`**: Sets bit 0 to 1, enabling receive data available interrupt
- **Why enable receive interrupt?**
  - Notifies CPU when UART receives data
  - Enables real-time data processing
---

### Summary

| Step                | Register     | Value   | Function                                   |
|--------------------|--------------|---------|------------------------------------------|
| 1. Disable all interrupts | IER    | `0x00`  | Disable all interrupts to ensure initialization process is not interrupted |
| 2. Set baud rate   | LCR          | `0x80`  | Enable access to baud rate divisor registers |
|                    | DLL          | `0x03`  | Set low byte of baud rate divisor         |
|                    | DLM          | `0x00`  | Set high byte of baud rate divisor        |
| 3. Configure transmission format | LCR | `0x03` | Set data format to 8 data bits, 1 stop bit, no parity |
| 4. Enable and clear FIFO | FCR    | `0x07`  | Enable FIFO and clear receive/transmit buffers |
| 5. Enable receive interrupt | IER  | `0x01`  | Enable receive data available interrupt    |

---

With this initialization configuration, UART will be configured to work at 38,400 bps baud rate, 8 data bits, 1 stop bit, no parity, with receive interrupt and FIFO buffer enabled.

Then we can perform read and write operations:

```bash
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
```

Using these two masks:

```bash
// Line Status Register bits
const LSR_DR: u8 = 1 << 0;   // Data Ready
const LSR_THRE: u8 = 1 << 5; // Transmitter Holding Register Empty
```

We can use these masks to check the LSR register and then complete reading from the RBR register and writing to the THR register. We can only write new data when THR is empty. This prevents data loss or overwriting and ensures correct data transmission. The mask settings are based on the LSR register bit definitions:

---

| Bit | Name                        | Description                                                           |
|-----|-----------------------------|-----------------------------------------------------------------------|
| 0   | **Data Ready (DR)**         | Set to 1 when data is available in the receive buffer                  |
| 1   | Overrun Error (OE)          | Set to 1 when receive buffer overflow occurs                           |
| 2   | Parity Error (PE)           | Set to 1 when parity error is detected                                 |
| 3   | Framing Error (FE)          | Set to 1 when framing error is detected                                |
| 4   | Break Interrupt (BI)        | Set to 1 when break condition is detected                              |
| 5   | **Transmitter Holding Register Empty (THRE)** | Set to 1 when transmit holding register is empty     |
| 6   | Transmitter Empty (TEMT)    | Set to 1 when both transmit holding and shift registers are empty      |
| 7   | Error in RCVR FIFO (FIFOERR)| Set to 1 when there's an error in receive FIFO (FIFO mode only)       |

---

SBI is a Machine-mode operation that uses CPU general-purpose registers, which differs from UART hardware module:

---

| Feature           | SBI Registers                     | UART Registers                        |
|-------------------|-----------------------------------|---------------------------------------|
| **Register Type** | CPU general-purpose registers (e.g., `x10`, `x17`) | UART hardware control registers (MMIO) |
| **Access Method** | Direct access via assembly instructions | Access via Memory-mapped I/O (MMIO)    |
| **Purpose**       | Pass parameters, call numbers, and return values | Configure and control UART behavior    |
| **Address Space** | Registers are part of CPU, no address | Registers have fixed physical addresses (e.g., `0x1000_0000`) |

In the `sbi` implementation, the `sbi_call` function uses the `ecall` instruction to call Machine mode SBI services.

```bash
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x17") which,
        );
    }
    ret
}
```

The core implementation lies in this inline assembly code:

```bash
core::arch::asm!(
    "ecall",
    inlateout("x10") arg0 => ret,
    in("x11") arg1,
    in("x12") arg2,
    in("x17") which,
);
```

For the related ABI syntax, we can refer to this documentation: https://github.com/riscv-non-isa/riscv-elf-psabi-doc

#### **(1) `"ecall"`**
- After executing `ecall`, CPU switches to Machine mode and executes the corresponding functionality in the SBI implementation (like OpenSBI).

#### **(2) `inlateout("x10") arg0 => ret`**
- **`inlateout`**: Indicates `x10` register is both input and output.
- **`"x10"`**: Specifies use of `x10` register (i.e., `a0` register).
- **`arg0 => ret`**:
  - `arg0` is input value, passed to `x10` register.
  - `ret` is output value, read return value from `x10` register.

#### **(3) `in("x11") arg1`**
- **`in`**: Indicates `x11` register is input.
- **`"x11"`**: Specifies use of `x11` register (i.e., `a1` register).
- **`arg1`**: Passes `arg1` value to `x11` register.

#### **(4) `in("x12") arg2`**
- **`in`**: Indicates `x12` register is input.
- **`"x12"`**: Specifies use of `x12` register (i.e., `a2` register).
- **`arg2`**: Passes `arg2` value to `x12` register.

#### **(5) `in("x17") which`**
- **`in`**: Indicates `x17` register is input.
- **`"x17"`**: Specifies use of `x17` register (i.e., `a7` register).
- **`which`**: Passes `which` value (SBI call number) to `x17` register.

---

| Register | Alias | Purpose                  |
|----------|-------|--------------------------|
| `x10`    | `a0`  | Input parameter 1 / Return value |
| `x11`    | `a1`  | Input parameter 2        |
| `x12`    | `a2`  | Input parameter 3        |
| `x17`    | `a7`  | SBI call number         |

- **`x10`(a0)**:
  - Input: `arg0`
  - Output: `ret` (SBI call return value)
- **`x11`(a1)**:
  - Input: `arg1`
- **`x12`(a2)**:
  - Input: `arg2`
- **`x17`(a7)**:
  - Input: `which` (SBI call number)

---

Let's look at the SBI call process through `console_putchar`:

```bash
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}
```

1. **First set parameters**:
   - `which = SBI_CONSOLE_PUTCHAR` (call number 1)
   - `arg0 = c` (character to print)
   - `arg1 = 0` (unused)
   - `arg2 = 0` (unused)
2. **Then execute `ecall`**:
   - Switch to Machine mode, execute `console_putchar` functionality in SBI implementation
3. **Finally return result**:
   - Return value through `x10`(a0) register (unused)

---

Alright, we've covered both methods, now let's look at the encapsulation. The encapsulation method is the same as [blog_os](https://os.phil-opp.com/vga-text-mode/), except we're calling either `uart` or `sbi`. Both methods work - `uart` is more flexible and theoretically more efficient (as there's no overhead from switching from S mode to M mode), while the `sbi` implementation is much simpler but requires reading related documentation.

```bash
// src/console.rs

use core::fmt::{self, Write};
//use crate::sbi::console_putchar;
use crate::uart::putchar;

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            //console_putchar(c as usize);
            putchar(c as u8);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut stdout = $crate::console::Stdout;
        stdout.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    Stdout.write_fmt(args).unwrap();
}
```