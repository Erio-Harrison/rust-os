OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = BASE_ADDRESS;
    
    .text : {
        *(.text.entry)    /* entry code first */
        *(.text .text.*)
    }
    
    .rodata : ALIGN(16) {
        *(.rodata .rodata.*)
    }
    
    .data : ALIGN(16) {
        *(.data .data.*)
    }
    
    .bss : ALIGN(16) {
        *(.bss.stack)    /* stack section */
        *(.bss .bss.*)
    }
    
    . = ALIGN(4096);
    end = .;
}