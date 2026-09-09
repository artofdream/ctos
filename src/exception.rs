//! AArch64 EL1 exception vectors (FR-06 / M3).
//!
//! Installs `VBAR_EL1` with a 2 KiB-aligned table. Current-EL / SP_ELx
//! synchronous exceptions run a Rust handler; a `BRK` from AArch64 is
//! resumable. Other slots park (print + `wfe`, or semihosting fail in
//! tests). This is QEMU `virt` only — not a board claim.
//!
//! Context format and EL choice: ADR-004.

use core::arch::global_asm;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::println;

/// ESR_EL1.EC: BRK instruction in AArch64 state (ARM ARM).
const ESR_EC_BRK_A64: u64 = 0x3C;

static BRK_COUNT: AtomicU64 = AtomicU64::new(0);

/// GPR + exception-register frame for the current-EL sync path.
/// Layout must match `sync_current_elx` in the vector asm.
#[repr(C)]
#[allow(dead_code)]
pub struct ExceptionContext {
    pub x: [u64; 30],
    pub lr: u64,
    pub elr: u64,
    pub spsr: u64,
    pub esr: u64,
}

global_asm!(
    r#"
    .section .text.vectors, "ax"
    .align 11
    .global exception_vectors
exception_vectors:
    // Current EL, SP_EL0
    .align 7
    mov x0, #0x000
    b handle_unhandled_exception
    .align 7
    mov x0, #0x080
    b handle_unhandled_exception
    .align 7
    mov x0, #0x100
    b handle_unhandled_exception
    .align 7
    mov x0, #0x180
    b handle_unhandled_exception

    // Current EL, SP_ELx — sync is the live path (we use SPSel = 1)
    .align 7
    b sync_current_elx
    .align 7
    mov x0, #0x280
    b handle_unhandled_exception
    .align 7
    mov x0, #0x300
    b handle_unhandled_exception
    .align 7
    mov x0, #0x380
    b handle_unhandled_exception

    // Lower EL, AArch64 — park until we have EL0
    .align 7
    mov x0, #0x400
    b handle_unhandled_exception
    .align 7
    mov x0, #0x480
    b handle_unhandled_exception
    .align 7
    mov x0, #0x500
    b handle_unhandled_exception
    .align 7
    mov x0, #0x580
    b handle_unhandled_exception

    // Lower EL, AArch32 — park
    .align 7
    mov x0, #0x600
    b handle_unhandled_exception
    .align 7
    mov x0, #0x680
    b handle_unhandled_exception
    .align 7
    mov x0, #0x700
    b handle_unhandled_exception
    .align 7
    mov x0, #0x780
    b handle_unhandled_exception
    .align 11

    .section .text
    .align 2
sync_current_elx:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    mrs x0, elr_el1
    str x0, [sp, #248]
    mrs x0, spsr_el1
    str x0, [sp, #256]
    mrs x0, esr_el1
    str x0, [sp, #264]
    mov x0, sp
    bl handle_sync_exception
    ldr x0, [sp, #248]
    msr elr_el1, x0
    ldr x0, [sp, #256]
    msr spsr_el1, x0
    ldr x30, [sp, #240]
    ldp x28, x29, [sp, #224]
    ldp x26, x27, [sp, #208]
    ldp x24, x25, [sp, #192]
    ldp x22, x23, [sp, #176]
    ldp x20, x21, [sp, #160]
    ldp x18, x19, [sp, #144]
    ldp x16, x17, [sp, #128]
    ldp x14, x15, [sp, #112]
    ldp x12, x13, [sp, #96]
    ldp x10, x11, [sp, #80]
    ldp x8, x9, [sp, #64]
    ldp x6, x7, [sp, #48]
    ldp x4, x5, [sp, #32]
    ldp x2, x3, [sp, #16]
    ldp x0, x1, [sp, #0]
    add sp, sp, #272
    eret

    .global ensure_el1
ensure_el1:
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #1
    b.eq 2f
    cmp x0, #2
    b.ne 2f
    mrs x0, hcr_el2
    orr x0, x0, #(1 << 31)
    msr hcr_el2, x0
    mov x0, sp
    msr sp_el1, x0
    mov x0, #0x3c5
    msr spsr_el2, x0
    adr x0, 1f
    msr elr_el2, x0
    isb
    eret
1:
2:
    ret
    "#
);

unsafe extern "C" {
    fn exception_vectors();
    fn ensure_el1();
}

pub fn vector_table_addr() -> u64 {
    exception_vectors as *const () as usize as u64
}

pub fn current_el() -> u64 {
    let raw: u64;
    unsafe {
        core::arch::asm!("mrs {el}, CurrentEL", el = out(reg) raw);
    }
    (raw >> 2) & 0b11
}

#[allow(dead_code)] // read back in `#[test_case]`; hello kernel only writes VBAR.
pub fn vbar_el1() -> u64 {
    let vbar: u64;
    unsafe {
        core::arch::asm!("mrs {v}, vbar_el1", v = out(reg) vbar);
    }
    vbar
}

#[allow(dead_code)] // asserted in `#[test_case]`.
pub fn brk_count() -> u64 {
    BRK_COUNT.load(Ordering::SeqCst)
}

/// Execute `BRK #0`. The current-EL sync handler skips the instruction.
pub fn breakpoint() {
    unsafe {
        core::arch::asm!("brk #0");
    }
}

fn park() -> ! {
    #[cfg(any(test, feature = "force-fail"))]
    crate::qemu::exit_failure();
    #[cfg(not(any(test, feature = "force-fail")))]
    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}

/// Drop to EL1 if needed, then point `VBAR_EL1` at the vector table.
pub fn init() {
    unsafe {
        ensure_el1();
    }
    let el = current_el();
    if el != 1 {
        println!("exception: need EL1, CurrentEL={}", el);
        park();
    }
    let vbar = vector_table_addr();
    if vbar & 0x7ff != 0 {
        println!("exception: VBAR misaligned {:#x}", vbar);
        park();
    }
    unsafe {
        core::arch::asm!(
            "msr vbar_el1, {v}",
            "isb",
            v = in(reg) vbar,
        );
    }
}

#[no_mangle]
pub extern "C" fn handle_sync_exception(ctx: &mut ExceptionContext) {
    let ec = (ctx.esr >> 26) & 0x3f;
    if ec == ESR_EC_BRK_A64 {
        BRK_COUNT.fetch_add(1, Ordering::SeqCst);
        println!(
            "exception: sync BRK esr={:#x} elr={:#x}",
            ctx.esr, ctx.elr
        );
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
    println!(
        "exception: unhandled sync esr={:#x} elr={:#x}",
        ctx.esr, ctx.elr
    );
    park();
}

#[no_mangle]
pub extern "C" fn handle_unhandled_exception(kind: u64) -> ! {
    println!("exception: unhandled vector {:#x}", kind);
    park();
}

#[cfg(test)]
#[test_case]
fn vbar_el1_points_at_table() {
    assert_eq!(current_el(), 1);
    assert_eq!(vbar_el1(), vector_table_addr());
}

#[cfg(test)]
#[test_case]
fn breakpoint_from_current_el() {
    let before = brk_count();
    breakpoint();
    assert_eq!(brk_count(), before + 1);
}
