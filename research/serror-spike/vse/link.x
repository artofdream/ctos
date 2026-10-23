ENTRY(_entry)
SECTIONS {
  . = 0x40080000;
  .text : { KEEP(*(.text.boot)) *(.text*) }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  .bss : { *(.bss*) }
  . = ALIGN(16);
  . += 0x4000;
  stack1_top = .;
}
