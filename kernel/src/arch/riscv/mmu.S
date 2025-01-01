.global setup_paging
setup_paging:
    # a0: 页表基地址
    
    # 设置 satp 寄存器
    srl a0, a0, 12
    li t0, 8 << 60
    or a0, a0, t0
    csrw satp, a0
    
    # 刷新 TLB
    sfence.vma zero, zero
    ret