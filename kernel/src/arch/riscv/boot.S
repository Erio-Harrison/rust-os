.section .text.init
.global _start
_start:
    # 设置栈指针
    la sp, __kernel_stack_end
    
    # 清理 BSS 段
    la a0, __bss_start
    la a1, __bss_end
    call clear_bss
    
    # 初始化页表
    la a0, __kernel_page_table
    call setup_paging
    
    # 跳转到 Rust 代码
    tail kernel_main

.align 4
clear_bss:
    beq a0, a1, 2f
1:  
    sd zero, (a0)
    addi a0, a0, 8
    blt a0, a1, 1b
2:  
    ret