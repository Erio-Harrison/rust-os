# Constants definitions
.equ TRAMPOLINE, 0x3ffffff000
.equ TRAPFRAME,  0x3fffffe000
.equ PGSIZE,     4096
.equ PGSHIFT,    12

# CSR register definitions
.equ MSTATUS_MPP_MASK,  0x1800
.equ MSTATUS_MPP_S,     0x0800
.equ MSTATUS_MPP_U,     0x0000
.equ MSTATUS_MIE,       0x0008

.equ SIE_SEIE,          0x0200
.equ SIE_STIE,          0x0020
.equ SIE_SSIE,          0x0002

.equ MIE_MEIE,          0x0800
.equ MIE_MTIE,          0x0080
.equ MIE_MSIE,          0x0008

.equ SSTATUS_SPP,       0x00000100
.equ SSTATUS_SPIE,      0x00000020
.equ SSTATUS_SIE,       0x00000002