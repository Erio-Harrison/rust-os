Basic Toolchain Installation:
```bash
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src
rustup component add llvm-tools-preview
```

In x86 systems, we use firmware standards like BIOS or UEFI, while in RISC-V systems, we use the SBI standard. When targeting actual RISC-V hardware (like SiFive development boards), OpenSBI (a specific implementation of the SBI specification) typically needs to be flashed onto the hardware as firmware. However, since we're using QEMU to emulate the RISC-V environment, we don't need to install OpenSBI separately as it's already built into QEMU.

You can see the contents of `.cargo/config.toml`:
```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "default", "-kernel", "target/riscv64gc-unknown-none-elf/debug/rust-os"]
```

Here, `-bios` doesn't refer to the BIOS standard but rather is a QEMU parameter for specifying the firmware standard. We use `default`, which means QEMU's default firmware (OpenSBI). For beginners, QEMU's default BIOS is sufficient for most scenarios. If you need to debug or customize OpenSBI, you can download or compile it and explicitly specify its path.

For example:
```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "path/to/opensbi.bin", "-kernel", "target/riscv64gc-unknown-none-elf/debug/rust-os"]
```

When a computer receives power, the CPU starts executing firmware code from a fixed memory address - this is the firmware loading process.

The firmware then reads the bootloader (like GRUB or Windows Boot Manager) from the boot device (like hard drive or USB).

The bootloader reads the operating system's `kernel.bin` file and loads it into memory. Then, the bootloader transfers control to the kernel, and the operating system starts running.

The overall flow is like this:

![HowToRunOS](../../assets/HowToRunOS.png)

The `-kernel` parameter in `runner` specifies the kernel file to run (the compiled binary file, in our case `rust-os`). Interestingly, if we don't explicitly specify the kernel file:

```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "path/to/opensbi.bin", "-kernel"]
```

When running `cargo build` or `cargo run`, it still works because **cargo** automatically passes the generated binary file as the parameter for '-kernel' to QEMU.

The first two parameters are self-explanatory: `-machine virt` specifies that QEMU should use the `virt` virtual machine, which is a virtual RISC-V platform. `-nographic` disables graphical output and uses only the command-line interface. With `-nographic`, QEMU automatically redirects serial output to the command-line terminal. Since we haven't configured graphical output in our implementation, all output will be redirected to the command-line terminal.

Now we can start development. Our operating system will ultimately generate a binary file (`rust-os`) that runs on QEMU. The transformation process from source code to binary file is shown in the following diagram:

![sourceTobin](../../assets/sourceTObin.png)

At the beginning of our source code (`main.rs`), we embed assembly code through: `global_asm!(include_str!("arch/riscv/boot.S"));`:

```bash
    .section .text.entry
    .globl _start
_start:
    la      sp, boot_stack_top
    call    rust_main

    .section .bss.stack
    .globl boot_stack
boot_stack:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top:
```

In the configuration file, we specify the linker script location:

```bash
rustflags = ["-C", "link-arg=-Tsrc/arch/riscv/linker.ld"]
```

Its complete content is:

```bash
OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = BASE_ADDRESS;
    skernel = .;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
    }
}
```

The complete workflow is as follows (corresponding to the above diagrams):

1. Compilation Stage:
   - `boot.S` is compiled into an object file (`boot.o`), containing `.text.entry` and `.bss.stack` sections.
   - Rust code is compiled into object files (like `main.o`), containing `.text`, `.rodata`, `.data`, and `.bss` sections.

2. Linking Stage:
   - The linker combines object files into an executable file (like `kernel.bin`) according to `linker.ld`.
   - The `.text.entry` section is placed at the beginning of the `.text` section, ensuring the `_start` symbol is at the start of the code section.
   - The `.bss.stack` section is placed in the `.bss` section, with stack space properly allocated.

3. Runtime Stage:
   - QEMU loads the executable file into memory (starting at `0x80200000`).
   - The bootloader jumps to the `_start` symbol, beginning the operating system's startup code.
   - The code in `boot.S` sets up the stack pointer and jumps to the Rust code, starting the operating system's main logic.

With this, we've established the basic framework of our operating system. In the next section, we'll implement the basic `print` functionality.