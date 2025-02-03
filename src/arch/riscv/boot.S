.section .text.entry
    .globl _start
_start:
    # Set up a stack for C.
    # hart_id is used to calculate the stack pointer offset for each CPU.
    # Each hart gets its own 4096-byte stack.
    csrr a1, mhartid          # Get hart ID (CPU number)
    li a0, 4096               # Stack size (4096 bytes)
    mul a0, a0, a1            # Multiply stack size by hart ID to get the offset
    la sp, boot_stack_top     # Load base address of boot stack
    add sp, sp, a0            # Add the hart offset to base stack address

    # Call start() to transition to the kernel
    call start

.section .bss.stack
    .globl boot_stack
boot_stack:
    .space 4096 * 16           # Allocate space for 16 stacks (16 CPUs)
    .globl boot_stack_top
boot_stack_top:
