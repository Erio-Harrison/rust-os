我们来实现基本的打印功能，正式开始前，需要先介绍下RISC-V 的特权级别。

RISC-V 定义了四个主要的特权级别：

1. Machine 模式 (M 模式)：

最高特权级别，直接访问硬件。

通常由固件（如 OpenSBI）或操作系统内核的最低层使用。

2. Supervisor 模式 (S 模式)：

用于操作系统内核。

可以访问部分硬件资源，但需要通过 M 模式的接口（如 SBI）来访问更底层的硬件。

3. User 模式 (U 模式)：

用于应用程序。

无法直接访问硬件，必须通过系统调用（S 模式）或 SBI（M 模式）来访问硬件。

4. Hypervisor 模式 (H 模式):

位于 S 模式和 M 模式之间，具有高于 S 模式但低于 M 模式的特权，能够直接管理虚拟 CPU、内存等资源。

我们重点关注的是前三种。

我们在实现打印的时候用了两种方式: `UART` 和 `SBI`, `SBI`暂时被注释掉了：

```bash
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

`UART` 是 S 级别的操作， 操作是通过直接读写内存映射的寄存器来实现的。这些寄存器的地址是物理地址（如 UART_BASE_ADDR = 0x1000_0000， 它是QEMU 虚拟机的 UART 物理地址），操作系统内核（运行在 S 模式）可以直接访问这些地址。

`uart.rs`里的`read_reg` 和 `write_reg` 函数都是直接通过物理地址访问 UART 寄存器。

```bash
/// Read from a register
unsafe fn read_reg(reg: usize) -> u8 {
    read_volatile((UART_BASE_ADDR + reg) as *const u8)
}

/// Write to a register
unsafe fn write_reg(reg: usize, val: u8) {
    write_volatile((UART_BASE_ADDR + reg) as *mut u8, val)
}
```

这两个函数都是运行在S模式下，通过一种叫做 `MMIO`映射的内存地址来完成内存的读写, 可以看到下面这个示意图：

```
CPU地址空间
+------------------+ 0xFFFFFFFF
|      ...        |
+------------------+
|    外设寄存器     | <- MMIO区域（比如UART位于0x10000000）
+------------------+
|     内存         |
+------------------+ 0x00000000
```

CPU有一条统一的地址总线，地址解码器(Address Decoder)会判断访问的地址属于内存还是外设， 如果是外设地址，信号会被路由到对应的外设控制器， 外设控制器会处理这个读写请求。以 `write_volatile(0x10000000, 'A')` 为例，当CPU执行：`write_volatile(0x10000000, 'A')`， CPU会先将地址0x10000000放到地址总线，地址解码器识别这是UART的地址范围， 然后将写信号和数据'A'路由给UART控制器， UART控制器接收到信号，将字符'A'放入发送缓冲区。

根据QEMU模拟的NS16550A UART 设备寄存器布局，我们可以定义如下常量：

```bash
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

通过基址+偏移量（比如访问 LCR 寄存器的地址为 `UART_BASE_ADDR + LCR`，即 `0x1000_0000 + 0x3 = 0x1000_0003`。），我们可以得到这些寄存器的位置，注意它们都是硬编码的，因为这些偏移量是 NS16550A UART 的硬件特性，由芯片的寄存器布局决定。

---

| 偏移量（Offset） | 寄存器名称                        | 访问方式 | 描述                                   |
|------------------|-----------------------------------|----------|----------------------------------------|
| `0x0`            | **RBR (Receiver Buffer Register)** | 读       | 接收缓冲区寄存器，用于读取接收到的数据。 |
| `0x0`            | **THR (Transmitter Holding Register)** | 写       | 发送保持寄存器，用于写入要发送的数据。   |
| `0x0`            | **DLL (Divisor Latch Low)**        | 读/写    | 波特率除数锁存器低字节（DLAB=1 时访问）。|
| `0x1`            | **DLM (Divisor Latch High)**       | 读/写    | 波特率除数锁存器高字节（DLAB=1 时访问）。|
| `0x1`            | **IER (Interrupt Enable Register)** | 读/写    | 中断使能寄存器，用于控制中断。           |
| `0x2`            | **FCR (FIFO Control Register)**    | 写       | FIFO 控制寄存器，用于控制 FIFO 缓冲区。  |
| `0x3`            | **LCR (Line Control Register)**    | 读/写    | 线路控制寄存器，用于配置数据格式。       |
| `0x5`            | **LSR (Line Status Register)**     | 读       | 线路状态寄存器，用于检查 UART 状态。     |

---

在正式开始传输数据之前，我们需要先进行UART的初始化，设置寄存器的相关参数：

```bash
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

### 1. **禁用所有中断**
```rust
write_reg(IER, 0x00);
```
- **IER (Interrupt Enable Register)**：中断使能寄存器，用于控制 UART 的中断。
- **`0x00`**：将 IER 寄存器的所有位设置为 0，禁用所有中断。
- **为什么需要禁用中断？**
  - 在初始化过程中，禁用中断可以避免在配置 UART 时被不必要的中断打断。
  - 确保初始化过程的原子性。

---

### 2. **设置波特率**
```rust
write_reg(LCR, 0x80);
write_reg(DLL, 0x03);
write_reg(DLM, 0x00);
```
- **LCR (Line Control Register)**：线路控制寄存器，用于配置数据格式和访问波特率除数寄存器。
- **DLAB (Divisor Latch Access Bit)**：LCR 寄存器的第 7 位，用于控制是否访问波特率除数寄存器。
  - **`0x80`**：将 DLAB 位设置为 1，使能访问波特率除数寄存器（DLL 和 DLM）。
- **DLL (Divisor Latch Low)** 和 **DLM (Divisor Latch High)**：波特率除数寄存器，用于设置波特率。
  - 波特率计算公式：`波特率 = 时钟频率 / (16 * 除数)`。
  - **`DLL = 0x03` 和 `DLM = 0x00`**：设置除数为 3，假设时钟频率为 1.8432 MHz，则波特率为 `1.8432 MHz / (16 * 3) = 38,400 bps`。
- **为什么需要设置波特率？**
  - 波特率决定了数据传输的速度。发送端和接收端必须使用相同的波特率才能正确通信。

---

### 3. **配置传输格式**
```rust
write_reg(LCR, 0x03);
```
- **LCR (Line Control Register)**：线路控制寄存器，用于配置数据格式。
- **`0x03`**：
  - 第 0-1 位：设置数据位为 8 位。
  - 第 2 位：设置停止位为 1 位。
  - 第 3 位：禁用奇偶校验。
- **为什么需要配置传输格式？**
  - 数据格式（数据位、停止位、奇偶校验）必须与通信的另一端一致，否则数据传输会出错。

---

### 4. **使能和清空 FIFO**
```rust
write_reg(FCR, 0x07);
```
- **FCR (FIFO Control Register)**：FIFO 控制寄存器，用于控制 FIFO 缓冲区的行为。
- **`0x07`**：
  - 第 0 位：使能 FIFO。
  - 第 1-2 位：清空接收和发送 FIFO。
  - 第 3 位：保留位，通常设置为 0。
- **为什么需要使能和清空 FIFO？**
  - FIFO（先进先出）缓冲区用于临时存储接收和发送的数据，提高数据传输的效率。
  - 清空 FIFO 可以确保初始化时缓冲区中没有残留的旧数据。

---

### 5. **使能接收中断**
```rust
write_reg(IER, 0x01);
```
- **IER (Interrupt Enable Register)**：中断使能寄存器，用于控制 UART 的中断。
- **`0x01`**：将第 0 位设置为 1，使能接收数据可用中断。
- **为什么需要使能接收中断？**
  - 当 UART 接收到数据时，触发中断通知 CPU 处理数据。
  - 使能接收中断可以提高数据处理的实时性。

---

### 总结

| 步骤               | 寄存器       | 值      | 功能                                   |
|--------------------|--------------|---------|----------------------------------------|
| 1. 禁用所有中断     | IER          | `0x00`  | 禁用所有中断，确保初始化过程不被中断打断。 |
| 2. 设置波特率       | LCR          | `0x80`  | 使能访问波特率除数寄存器。               |
|                    | DLL          | `0x03`  | 设置波特率除数的低字节。                 |
|                    | DLM          | `0x00`  | 设置波特率除数的高字节。                 |
| 3. 配置传输格式     | LCR          | `0x03`  | 设置数据格式为 8 位数据位、1 位停止位、无奇偶校验。 |
| 4. 使能和清空 FIFO  | FCR          | `0x07`  | 使能 FIFO 并清空接收和发送缓冲区。       |
| 5. 使能接收中断     | IER          | `0x01`  | 使能接收数据可用中断。                   |

---

这样的初始化配置，UART 会被配置为以 38,400 bps 的波特率、8 位数据位、1 位停止位、无奇偶校验的方式工作，并启用了接收中断和 FIFO 缓冲区。

然后我们就可以进行读写操作了：

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

借助两个掩码：

```bash
// Line Status Register bits
const LSR_DR: u8 = 1 << 0;   // Data Ready
const LSR_THRE: u8 = 1 << 5; // Transmitter Holding Register Empty
```
我们可以利用这两个掩码实现对LSR寄存器的检查，再分别完成对RBR寄存器的读和THR寄存器的写，只有当 THR 为空时，才能写入新的数据。这是为了避免数据丢失或覆盖，确保数据能够正确发送。掩码设置是由于LSR寄存器的位定义：

---

| 位（Bit） | 名称                        | 描述                                                                 |
|-----------|-----------------------------|----------------------------------------------------------------------|
| 0         | **Data Ready (DR)**         | 当接收缓冲区中有数据时，该位为 1。                                    |
| 1         | Overrun Error (OE)          | 当接收缓冲区溢出时，该位为 1。                                        |
| 2         | Parity Error (PE)           | 当检测到奇偶校验错误时，该位为 1。                                    |
| 3         | Framing Error (FE)          | 当检测到帧错误时，该位为 1。                                          |
| 4         | Break Interrupt (BI)        | 当检测到中断条件时，该位为 1。                                        |
| 5         | **Transmitter Holding Register Empty (THRE)** | 当发送保持寄存器为空时，该位为 1。                                    |
| 6         | Transmitter Empty (TEMT)    | 当发送保持寄存器和发送移位寄存器都为空时，该位为 1。                  |
| 7         | Error in RCVR FIFO (FIFOERR)| 当接收 FIFO 中有错误时，该位为 1（仅适用于 FIFO 模式）。              |

---

SBI 是 M 级别的操作， 它使用的寄存器是CPU的通用寄存器，和 UART 硬件模块不同：

---

| 特性               | SBI 使用的寄存器                  | UART 使用的寄存器                     |
|--------------------|-----------------------------------|----------------------------------------|
| **寄存器类型**      | CPU 的通用寄存器（如 `x10`、`x17`）。 | UART 硬件设备的控制寄存器（MMIO）。     |
| **访问方式**        | 通过汇编指令直接访问。             | 通过内存映射 I/O（MMIO）访问。          |
| **用途**            | 传递参数、调用号和返回值。         | 配置和控制 UART 的行为。               |
| **地址空间**        | 寄存器是 CPU 的一部分，没有地址。  | 寄存器有固定的物理地址（如 `0x1000_0000`）。 |

在`sbi`的实现里，`sbi_call` 函数通过 `ecall` 指令调用 M 模式的 SBI 服务。

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

实现核心在于这段内联汇编代码：

```bash
core::arch::asm!(
    "ecall",
    inlateout("x10") arg0 => ret,
    in("x11") arg1,
    in("x12") arg2,
    in("x17") which,
);
```

相关ABI的语法，我们可以参考这个文档： https://github.com/riscv-non-isa/riscv-elf-psabi-doc

#### **(1) `"ecall"`**
- 执行 `ecall` 后，CPU 会切换到 Machine 模式，并执行 SBI 实现（如 OpenSBI）中对应的功能。

#### **(2) `inlateout("x10") arg0 => ret`**
- **`inlateout`**：表示 `x10` 寄存器既是输入也是输出。
- **`"x10"`**：指定使用 `x10` 寄存器（即 `a0` 寄存器）。
- **`arg0 => ret`**：
  - `arg0` 是输入值，传递给 `x10` 寄存器。
  - `ret` 是输出值，从 `x10` 寄存器读取返回值。

#### **(3) `in("x11") arg1`**
- **`in`**：表示 `x11` 寄存器是输入。
- **`"x11"`**：指定使用 `x11` 寄存器（即 `a1` 寄存器）。
- **`arg1`**：把 `arg1` 的值传递给 `x11` 寄存器。

#### **(4) `in("x12") arg2`**
- **`in`**：表示 `x12` 寄存器是输入。
- **`"x12"`**：指定使用 `x12` 寄存器（即 `a2` 寄存器）。
- **`arg2`**：把 `arg2` 的值传递给 `x12` 寄存器。

#### **(5) `in("x17") which`**
- **`in`**：表示 `x17` 寄存器是输入。
- **`"x17"`**：指定使用 `x17` 寄存器（即 `a7` 寄存器）。
- **`which`**：把 `which` 的值（SBI 调用号）传递给 `x17` 寄存器。

---


| 寄存器 | 别名 | 用途                     |
|--------|------|--------------------------|
| `x10`  | `a0` | 输入参数 1 / 返回值       |
| `x11`  | `a1` | 输入参数 2                |
| `x12`  | `a2` | 输入参数 3                |
| `x17`  | `a7` | SBI 调用号                |

- **`x10`（a0）**：
  - 输入：`arg0`。
  - 输出：`ret`（SBI 调用的返回值）。
- **`x11`（a1）**：
  - 输入：`arg1`。
- **`x12`（a2）**：
  - 输入：`arg2`。
- **`x17`（a7）**：
  - 输入：`which`（SBI 调用号）。

---

通过 `console_putchar` 来具体看下 SBI 调用的过程：

```bash
pub fn console_putchar(c: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, c, 0, 0);
}
```

1. **首先设置参数**：
   - `which = SBI_CONSOLE_PUTCHAR`（调用号 1）。
   - `arg0 = c`（要打印的字符）。
   - `arg1 = 0`（未使用）。
   - `arg2 = 0`（未使用）。
2. **然后执行 `ecall`**：
   - 切换到 Machine 模式，执行 SBI 实现中的 `console_putchar` 功能。
3. **最后返回结果**：
   - 返回值通过 `x10`（a0）寄存器返回（未使用）。
---

好的，两种方式到这里就介绍完了，然后就是封装。封装方式和 [blog_os](https://os.phil-opp.com/vga-text-mode/)是一样的，只是我们调用的是 `uart`或者 `sbi`。两种方法都可以，`uart`要更灵活，理论上也更高效（因为没有S模式切换到M模式的开销），`sbi`实现方式要简单很多，要读一下相关文档。

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