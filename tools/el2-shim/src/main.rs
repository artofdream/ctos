//! ADR-111 (N2): minimal EL2 shim for a **virtual SError** on stock QEMU.
//!
//! `-M virt,virtualization=on -device loader,file=<this ELF>,cpu-num=0` starts
//! core 0 here at EL2. The shim sets HCR_EL2 = RW | AMO (physical SError
//! routed to EL2, virtual SError enabled for EL1/EL0; IRQ/FIQ stay at EL1),
//! opens the EL1 timer/counter and FP, mirrors MIDR/MPIDR, then ERETs to the
//! ctos image at EL1h with DAIF masked. `_start` (0x4008_0000) is untouched:
//! the shim only jumps to it with x0..x3 as QEMU set them.
//!
//! EL1 asks through `hvc #0xC7` (x0 = command; only x0 is changed):
//!   0 = query  -> x0 = 0x5E2E_1D00 (shim present)
//!   1 = set HCR_EL2.VSE (virtual SError pending) -> x0 = 0
//!   2 = clear HCR_EL2.VSE                        -> x0 = 0
//! Any other HVC (ctos PSCI `hvc #0`) is forwarded as `smc #0` to QEMU's PSCI.
//! Anything else taken at EL2 prints `el2-shim: unexpected` and parks.
//! No RAM is used: the only scratch is TPIDR_EL2.
#![no_std]
#![no_main]

core::arch::global_asm!(r#"
.section .text.shim,"ax"
.global shim_entry
shim_entry:
    mrs x9, CurrentEL
    cmp x9, #8
    b.ne shim_park
    adr x9, shim_vec
    msr vbar_el2, x9
    movz x9, #0x0020
    movk x9, #0x8000, lsl #16
    msr hcr_el2, x9
    mov x9, #3
    msr cnthctl_el2, x9
    msr cntvoff_el2, xzr
    mov x9, #0x33ff
    msr cptr_el2, x9
    msr mdcr_el2, xzr
    mrs x9, midr_el1
    msr vpidr_el2, x9
    mrs x9, mpidr_el1
    msr vmpidr_el2, x9
    // SCTLR_EL1 is left at its reset value (nTWI/nTWE set), as without EL2.
    mov x9, #0x3c5
    msr spsr_el2, x9
    movz x9, #0x0000
    movk x9, #0x4008, lsl #16
    msr elr_el2, x9
    // UART cue (PL011 DR) so a log shows the shim ran: "el2-shim: el1\n"
    movz x10, #0x0000
    movk x10, #0x0900, lsl #16
    adr x11, shim_msg_el1
1:  ldrb w12, [x11], #1
    cbz w12, 2f
    str w12, [x10]
    b 1b
2:  isb
    eret

shim_park:
    wfe
    b shim_park

shim_unexpected:
    movz x10, #0x0000
    movk x10, #0x0900, lsl #16
    adr x11, shim_msg_bad
3:  ldrb w12, [x11], #1
    cbz w12, shim_park
    str w12, [x10]
    b 3b

shim_msg_el1:
    .asciz "el2-shim: el1\n"
shim_msg_bad:
    .asciz "el2-shim: unexpected\n"

.balign 2048
shim_vec:
.rept 8
.balign 0x80
    b shim_unexpected
.endr
// Lower EL AArch64: sync at +0x400
.balign 0x80
    msr tpidr_el2, x9
    mrs x9, esr_el2
    lsr x9, x9, #26
    cmp x9, #0x16
    b.ne shim_unexpected
    mrs x9, esr_el2
    and x9, x9, #0xffff
    cmp x9, #0xc7
    b.ne shim_psci
    cbz x0, shim_query
    cmp x0, #1
    b.eq shim_set
    cmp x0, #2
    b.eq shim_clr
    mov x0, #-1
    b shim_out
.balign 0x80
    b shim_unexpected   // lower EL IRQ (IMO=0: never routed here)
.balign 0x80
    b shim_unexpected   // lower EL FIQ
.balign 0x80
    b shim_unexpected   // lower EL SError (physical, AMO=1)
.rept 4
.balign 0x80
    b shim_unexpected
.endr

shim_query:
    movz x0, #0x1d00
    movk x0, #0x5e2e, lsl #16
    b shim_out
shim_set:
    mrs x9, hcr_el2
    orr x9, x9, #(1 << 8)
    msr hcr_el2, x9
    mov x0, #0
    b shim_out
shim_clr:
    mrs x9, hcr_el2
    bic x9, x9, #(1 << 8)
    msr hcr_el2, x9
    mov x0, #0
shim_out:
    mrs x9, tpidr_el2
    isb
    eret
shim_psci:
    mrs x9, tpidr_el2
    smc #0
    eret
"#);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
