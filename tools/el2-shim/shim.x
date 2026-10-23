/* ADR-111: EL2 shim. Lives in the virt flash window (PA 0), far from the
 * ctos image and RAM; loaded with `-device loader,file=…,cpu-num=0`, which
 * also sets core 0's reset PC to `shim_entry`. */
ENTRY(shim_entry)
SECTIONS {
  . = 0x0;
  .text : { KEEP(*(.text.shim)) *(.text*) }
  /DISCARD/ : { *(.data*) *(.bss*) *(.rodata*) *(.eh_frame*) *(.comment*) }
}
