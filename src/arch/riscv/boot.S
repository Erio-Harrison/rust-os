.section .text.entry
.global _entry

_entry:
    # set up a stack for C.
    la sp, stack0
    li a0, 1024*4
    csrr a1, mhartid
    addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0

    # jump to start() in start.rs
    call start

spin:
    j spin

.section .bss
.align 12
.global stack0
stack0:
    .space 4096 * 8  # 8 CPUs, 4KB per CPU
.global stack0_top
stack0_top: