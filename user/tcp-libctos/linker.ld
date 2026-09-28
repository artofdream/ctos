/* EL0 tcp-libctos payload. Load VA must match paging::EL0_PAGE (0x80002000). */
/* ADR-097: `.rodata` shares the ELF-header page at 0x80000000 (the
   allowlisted `app-hdr` slot, mapped EL0 read-only + NX). `.text` is alone
   on EL0_PAGE, which the loader maps EL0 execute-only (EL0 cannot read it).
   Headers + `.rodata` must fit in that one 4 KiB page; the loader refuses an
   image that puts read-only data on the text page or outside these slots. */
ENTRY(_start)

SECTIONS
{
    . = 0x80000000 + SIZEOF_HEADERS;

    .rodata : {
        *(.rodata .rodata.*)
    }

    . = 0x80002000;

    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }

    .data : {
        *(.data .data.*)
    }

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
        *(.ARM.exidx*)
    }
}
