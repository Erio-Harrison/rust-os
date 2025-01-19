.section .text.entry
.globl _start

_start:
    # 设置栈指针
    la sp, boot_stack_top
    
    # 存储返回地址
    addi sp, sp, -16
    sd ra, 8(sp)
    
    # 直接跳转到start
    tail start      # 使用tail call 而不是普通的call

.section .bss.stack
.align 12
.globl boot_stack
boot_stack:
    .space 4096 * 16
.globl boot_stack_top
boot_stack_top: