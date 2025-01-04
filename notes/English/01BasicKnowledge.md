In x86 systems, the firmware standards we use are BIOS or UEFI. In RISC-V systems, the standard we use is SBI (Supervisor Binary Interface). If our target platform is real RISC-V hardware (such as a SiFive development board), we typically need to flash OpenSBI (a specific implementation of the SBI specification) as firmware onto the hardware. However, since we are using QEMU to emulate a RISC-V environment, we do not need to install OpenSBI separately, as QEMU already has OpenSBI built-in. You can see the content in `.cargo/config.toml`:

```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "default", "-kernel", "target/riscv64gc-unknown-none-elf/debug/rust-os"]
```

Here, the `-bios` parameter does not refer to the BIOS standard but is a QEMU-specific parameter for specifying the firmware. We are using `default`, which means QEMU's default firmware, OpenSBI. For beginner development, QEMU's default BIOS is sufficient to support most scenarios. If you need to debug or customize OpenSBI, you can download or compile OpenSBI and explicitly specify its path.

For example:

```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "path/to/opensbi.bin", "-kernel", "target/riscv64gc-unknown-none-elf/debug/rust-os"]
```

When the computer is powered on, the CPU starts executing firmware code from a fixed memory address. This process is known as firmware loading.

The firmware then reads the bootloader (e.g., GRUB, Windows Boot Manager) from a boot device (such as a hard drive or USB drive).

The bootloader reads the operating system's `kernel.bin` file and loads it into memory. The bootloader then transfers control to the kernel, and the operating system starts running.

The overall process is roughly as follows:

![HowToRunOS](../../assets/HowToRunOS.png)

The `-kernel` parameter in `runner` specifies the kernel file to run (i.e., the compiled binary file, which in our case is `rust-os`). There is an interesting point here: if we do not explicitly specify the kernel file to run:

```bash
runner = ["qemu-system-riscv64", "-machine", "virt", "-nographic", "-bios", "path/to/opensbi.bin", "-kernel"]
```

When running `cargo build` or `cargo run`, it can still work correctly because **Cargo** automatically passes the generated binary file as the `-kernel` parameter to QEMU.

The first two parameters are relatively self-explanatory: `-machine virt` specifies that QEMU should use the `virt` virtual machine, which is a virtual RISC-V platform. `-nographic` disables graphical output and only uses the command-line interface. Here, we use `-nographic`, and QEMU will automatically redirect serial output to the command-line terminal. Since we have not configured graphical output in our implementation, all relevant output will be redirected to the command-line terminal for display.