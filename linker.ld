/* QEMU virt RAM starts at 0x40000000. TEXT_OFFSET 0x80000 matches the
   conventional Image load address used by -kernel on this machine.
   This script does not claim Raspberry Pi or other board layouts. */
ENTRY(_start)

SECTIONS
{
    . = 0x40080000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .rodata : {
        *(.rodata .rodata.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        . = ALIGN(8);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }

    . = ALIGN(16);
    __stack_bottom = .;
    . += 64K;
    __stack_top = .;
}
