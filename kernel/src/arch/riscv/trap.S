.align 4
.global trap_vector
trap_vector:
    # 保存上下文
    addi sp, sp, -256
    
    # 保存通用寄存器
    sd ra, 0(sp)
    sd sp, 8(sp)
    sd gp, 16(sp)
    # ... 保存其他寄存器
    
    # 调用 Rust trap 处理函数
    call trap_handler
    
    # 恢复寄存器
    ld ra, 0(sp)
    ld sp, 8(sp)
    ld gp, 16(sp)
    # ... 恢复其他寄存器
    
    addi sp, sp, 256
    sret