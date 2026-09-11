/* QEMU virt RAM starts at 0x40000000. TEXT_OFFSET 0x80000 matches the
   conventional Image load address used by -kernel on this machine.
   This script does not claim Raspberry Pi or other board layouts. */
ENTRY(_start)

SECTIONS
{
    . = 0x40080000;

    .text : {
        KEEP(*(.text.boot))
        . = ALIGN(2048);
        KEEP(*(.text.vectors))
        *(.text .text.*)
    }

    /* Page-align so ADR-019 can unmap identity .text after the
       0x40080000 boot stub without sharing a leaf with .rodata.
       rustc stores absolute string / vtable pointers in .rodata;
       those stay identity-mapped. */
    . = ALIGN(4096);
    __text_end = .;

    .rodata : {
        *(.rodata .rodata.*)
    }

    /* Dedicated 4 KiB identity text page (ADR-018). After MMU + high
       VBAR this page is unmapped from TTBR0; EL1 keeps fetching the
       twin via TTBR1. `_start` at 0x40080000 stays mapped. */
    . = ALIGN(4096);
    __ident_tear_start = .;
    .ident_tear : {
        KEEP(*(ident_tear))
        . = ALIGN(4096);
    }
    __ident_tear_end = .;

    /* Page-aligned so ADR-015 can map .text/.rodata RO+X and
       .data/.bss/stacks RW+NX without sharing a 4 KiB page.
       Sit in the *next* 2 MiB L2 block (0x4020_0000+) so KERNEL_TEXT
       (0x4008_0000, first RAM 2 MiB) and .data cannot share one L3.
       A fresh nightly that grows .text/.rodata used to overwrite that
       single L3 and miss paging/heap/el0 (cts-ai Docker on b2bbb99). */
    . = ALIGN(4096);
    . = 0x40201000;
    __data_start = .;

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

    /* 4 KiB unmapped holes (ADR-014). Punched after the MMU tables are
       filled so a downward overflow becomes a translation fault.
       Live stack pages are RW+NX with .data/.bss (ADR-015), not X+W. */
    . = ALIGN(4096);
    __stack_guard = .;
    . += 4096;
    __stack_bottom = .;
    . += 64K;
    __stack_top = .;

    /* Dedicated current-EL exception stack (SP_EL1). Not the thread stack. */
    . = ALIGN(4096);
    __exc_stack_guard = .;
    . += 4096;
    __exc_stack_bottom = .;
    . += 16K;
    __exc_stack_top = .;

    /* Nested / fatal path only. Switched in asm before any frame stores. */
    . = ALIGN(4096);
    __fatal_stack_guard = .;
    . += 4096;
    __fatal_stack_bottom = .;
    . += 8K;
    __fatal_stack_top = .;

    /* First allocatable frame is the next 4 KiB after image + stacks.
       QEMU virt DTB sits at RAM base (0x40000000), below this image. */
    . = ALIGN(4096);
    __kernel_end = .;
}
