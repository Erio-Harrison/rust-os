.section .text
    .globl __alltraps
    .align 2
__alltraps:
    # 保存原始栈指针
    csrw    sscratch, sp
    
    # 分配一个新的内核栈（使用一个预定义的内核栈地址）
    li      sp, 0x80210000    # 使用一个预定义的内核栈地址
    
    # 分配栈空间
    addi    sp, sp, -34*8
    
    # 保存通用寄存器，使用 t6 作为临时寄存器保存原始 sp
    sd      x1, 0*8(sp)
    csrr    t6, sscratch  
    sd      t6, 1*8(sp)
    sd      x3, 2*8(sp)
    sd      x4, 3*8(sp)
    sd      x5, 4*8(sp)
    sd      x6, 5*8(sp)
    sd      x7, 6*8(sp)
    sd      x8, 7*8(sp)
    sd      x9, 8*8(sp)
    sd      x10, 9*8(sp)
    sd      x11, 10*8(sp)
    sd      x12, 11*8(sp)
    sd      x13, 12*8(sp)
    sd      x14, 13*8(sp)
    sd      x15, 14*8(sp)
    sd      x16, 15*8(sp)
    sd      x17, 16*8(sp)
    sd      x18, 17*8(sp)
    sd      x19, 18*8(sp)
    sd      x20, 19*8(sp)
    sd      x21, 20*8(sp)
    sd      x22, 21*8(sp)
    sd      x23, 22*8(sp)
    sd      x24, 23*8(sp)
    sd      x25, 24*8(sp)
    sd      x26, 25*8(sp)
    sd      x27, 26*8(sp)
    sd      x28, 27*8(sp)
    sd      x29, 28*8(sp)
    sd      x30, 29*8(sp)
    sd      x31, 30*8(sp)
    
    # 保存 CSR
    csrr    t0, sstatus
    csrr    t1, sepc
    sd      t0, 31*8(sp)
    sd      t1, 32*8(sp)
    
    # 调用 Rust trap 处理函数
    mv      a0, sp
    call    trap_handler

    # 恢复 CSR
    ld      t0, 31*8(sp)
    ld      t1, 32*8(sp)
    csrw    sstatus, t0
    csrw    sepc, t1
    
    # 恢复通用寄存器
    ld      x1, 0*8(sp)
    ld      x3, 2*8(sp)
    ld      x4, 3*8(sp)
    ld      x5, 4*8(sp)
    ld      x6, 5*8(sp)
    ld      x7, 6*8(sp)
    ld      x8, 7*8(sp)
    ld      x9, 8*8(sp)
    ld      x10, 9*8(sp)
    ld      x11, 10*8(sp)
    ld      x12, 11*8(sp)
    ld      x13, 12*8(sp)
    ld      x14, 13*8(sp)
    ld      x15, 14*8(sp)
    ld      x16, 15*8(sp)
    ld      x17, 16*8(sp)
    ld      x18, 17*8(sp)
    ld      x19, 18*8(sp)
    ld      x20, 19*8(sp)
    ld      x21, 20*8(sp)
    ld      x22, 21*8(sp)
    ld      x23, 22*8(sp)
    ld      x24, 23*8(sp)
    ld      x25, 24*8(sp)
    ld      x26, 25*8(sp)
    ld      x27, 26*8(sp)
    ld      x28, 27*8(sp)
    ld      x29, 28*8(sp)
    ld      x30, 29*8(sp)
    ld      x31, 30*8(sp)
    
    # 恢复原始栈指针
    ld      sp, 1*8(sp)
    
    # 返回
    sret