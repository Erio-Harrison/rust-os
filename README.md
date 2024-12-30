# Rust OS Kernel

A minimal operating system kernel written in Rust, designed to run on QEMU. This educational project implements basic OS functionality including memory management, interrupt handling, keyboard input, and a simple task executor. 

I'm still working on it. My personal research project in the last semester is operating systems.

Reference: 

https://os.phil-opp.com/


![example](assets/example.png)


## Prerequisites

- Rust nightly toolchain
- QEMU (for running the kernel)
- `cargo-bootimage` (for creating bootable disk images)
- `llvm-tools-preview` component

## Building

1. Install the required tools:
```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview
cargo install bootimage
```

2. Build the kernel:
```bash
cargo build
```

3. Create a bootable disk image:
```bash
cargo bootimage
```

## Running

To run the kernel in QEMU:
```bash
cargo run
```

## Testing

Run the test suite:
```bash
cargo test
```

The project includes custom test frameworks and supports:
- Unit tests
- Integration tests
- Should-panic tests

## Technical Details

### Memory Layout
- Heap Start: 0x4444_4444_0000
- Heap Size: 100 KiB
- VGA Buffer: 0xb8000

### Hardware Support
- Architecture: x86_64
- Target: Custom bare metal target (`x86_64-blog_os.json`)
- Features: No MMX/SSE, using soft float

## Dependencies

- `bootloader`: Boot sector implementation
- `x86_64`: CPU instructions and register abstractions
- `spin`: Spinlocks for synchronization
- `volatile`: Safe volatile memory access
- `uart_16550`: Serial port driver
- `pic8259`: Programmable Interrupt Controller support
- `linked_list_allocator`: Heap allocation
- `lazy_static`: Static initialization with runtime values
- `pc-keyboard`: Keyboard input handling

## Project Structure

- `src/main.rs`: Kernel entry point and initialization
- `src/vga_buffer.rs`: VGA text mode implementation
- `src/interrupts.rs`: Interrupt handling
- `src/gdt.rs`: Global Descriptor Table setup
- `src/memory.rs`: Memory management
- `src/allocator.rs`: Heap allocation
- `src/task.rs`: Async task execution
- `src/serial.rs`: Serial port communication

## Acknowledgments

This kernel is inspired by Philipp Oppermann's ["Writing an OS in Rust"](https://os.phil-opp.com/) blog series.