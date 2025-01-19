OUTPUT_ARCH(riscv)
ENTRY(_entry)

BASE_ADDRESS = 0x80200000;

SECTIONS {
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    .rodata : {
        . = ALIGN(16);
        *(.rodata .rodata.*)
    }

    .data : {
        . = ALIGN(16);
        *(.data .data.*)
    }

    .bss : {
        . = ALIGN(16);
        stack0 = .;
        . = . + (4096 * 8);  /* 8 CPUs, 4KB per CPU */
        stack0_top = .;
        *(.bss .bss.*)
    }

    PROVIDE(end = .);
}