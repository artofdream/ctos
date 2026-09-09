//! AArch64 EL1 exception vectors (FR-06 / M3), fatal stack (FR-07 / M4),
//! and current-EL IRQ (FR-08 / M5).
//!
//! Installs `VBAR_EL1` with a 2 KiB-aligned table. After init the kernel
//! runs on `SP_EL0` (`SPSel = 0`); first-level current-EL exceptions use
//! `SP_EL1` (dedicated exception stack). A nested current-EL exception
//! (SP_ELx bank) switches to a third fatal stack before any stores, then
//! prints on the UART without the mutex and parks.
//!
//! AArch64 `BRK` from the thread stack is resumable. The hello kernel then
//! drops `SP_EL0` near the thread-stack floor and fires another `BRK` so
//! the first-level handler can nest — serial proof for FR-07.
//!
//! Without paging (M7) a stack overflow is not a hardware fault; it would
//! smash BSS. The dedicated stacks are the mitigation. The probe is a
//! nested `BRK` after a near-empty thread SP — honest for virt, not an
//! MMU guard-page claim. Context format and EL choice: ADR-004, ADR-005.

use core::arch::global_asm;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::println;
use crate::uart;

/// ESR_EL1.EC: BRK instruction in AArch64 state (ARM ARM).
const ESR_EC_BRK_A64: u64 = 0x3C;

static BRK_COUNT: AtomicU64 = AtomicU64::new(0);
static NEST_FATAL: AtomicBool = AtomicBool::new(false);

/// GPR + exception-register frame for the first-level current-EL sync path.
/// Layout must match `sync_current_el` in the vector asm.
#[repr(C)]
#[allow(dead_code)]
pub struct ExceptionContext {
    pub x: [u64; 30],
    pub lr: u64,
    pub elr: u64,
    pub spsr: u64,
    pub esr: u64,
}

unsafe extern "C" {
    static __stack_bottom: u8;
    static __stack_top: u8;
    static __exc_stack_bottom: u8;
    static __exc_stack_top: u8;
    static __fatal_stack_bottom: u8;
    static __fatal_stack_top: u8;
    fn exception_vectors();
    fn ensure_el1();
    #[allow(dead_code)] // hello kernel only; tests must not nest.
    fn trigger_fatal_nested_asm();
}

global_asm!(
    r#"
    .section .text.vectors, "ax"
    .align 11
    .global exception_vectors
exception_vectors:
    // Current EL, SP_EL0 — first-level (kernel runs with SPSel = 0)
    .align 7
    b sync_current_el
    .align 7
    b irq_current_el
    .align 7
    mov x0, #0x100
    b handle_unhandled_exception
    .align 7
    mov x0, #0x180
    b handle_unhandled_exception

    // Current EL, SP_ELx — already in a handler; do not store on SP_EL1
    .align 7
    mov x0, #0x200
    b fatal_enter
    .align 7
    mov x0, #0x280
    b fatal_enter
    .align 7
    mov x0, #0x300
    b fatal_enter
    .align 7
    mov x0, #0x380
    b fatal_enter

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
sync_current_el:
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

    irq_current_el:
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
    bl handle_irq
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

    // x0 = vector kind. Switch SP before any further stores.
    .global fatal_enter
fatal_enter:
    ldr x3, =__fatal_stack_top
    mov sp, x3
    mov x3, x0
    mrs x0, esr_el1
    mrs x1, elr_el1
    mov x2, x3
    bl handle_fatal_exception
1:
    wfe
    b 1b

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
    mrs x0, cnthctl_el2
    orr x0, x0, #3
    msr cnthctl_el2, x0
    msr cntvoff_el2, xzr
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

    // Hello-kernel FR-07 probe: abandon the thread stack, then BRK.
    // First exception uses SP_EL1. Handler nests a second BRK → fatal_enter.
    .global trigger_fatal_nested_asm
trigger_fatal_nested_asm:
    ldr x0, =__stack_bottom
    add x0, x0, #64
    mov sp, x0
    brk #0
    ldr x0, =__stack_top
    mov sp, x0
    b fatal_probe_missed
    "#
);

fn linker_sym(sym: *const u8) -> u64 {
    sym as usize as u64
}

pub fn thread_stack_bottom() -> u64 {
    linker_sym(core::ptr::addr_of!(__stack_bottom))
}

pub fn thread_stack_top() -> u64 {
    linker_sym(core::ptr::addr_of!(__stack_top))
}

pub fn exc_stack_bottom() -> u64 {
    linker_sym(core::ptr::addr_of!(__exc_stack_bottom))
}

pub fn exc_stack_top() -> u64 {
    linker_sym(core::ptr::addr_of!(__exc_stack_top))
}

pub fn fatal_stack_bottom() -> u64 {
    linker_sym(core::ptr::addr_of!(__fatal_stack_bottom))
}

pub fn fatal_stack_top() -> u64 {
    linker_sym(core::ptr::addr_of!(__fatal_stack_top))
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

#[cfg(test)]
fn spsel() -> u64 {
    let raw: u64;
    unsafe {
        core::arch::asm!("mrs {s}, SPSel", s = out(reg) raw);
    }
    raw & 1
}

#[cfg(test)]
fn current_sp() -> u64 {
    let sp: u64;
    unsafe {
        core::arch::asm!("mov {s}, sp", s = out(reg) sp);
    }
    sp
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

/// Execute `BRK #0`. The first-level current-EL sync handler skips the instruction.
pub fn breakpoint() {
    unsafe {
        core::arch::asm!("brk #0");
    }
}

/// Hello-kernel FR-07 probe: nest a `BRK` from a near-empty thread stack.
#[allow(dead_code)] // hello kernel only; tests must not nest.
pub fn trigger_fatal_nested() -> ! {
    NEST_FATAL.store(true, Ordering::SeqCst);
    unsafe {
        trigger_fatal_nested_asm();
    }
    fatal_probe_missed();
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

/// Drop to EL1 if needed, point `VBAR_EL1` at the table, then split stacks.
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
    let exc_top = exc_stack_top();
    if exc_top & 0xf != 0
        || fatal_stack_top() & 0xf != 0
        || thread_stack_top() & 0xf != 0
        || exc_stack_bottom() >= exc_top
        || fatal_stack_bottom() >= fatal_stack_top()
        || thread_stack_bottom() >= thread_stack_top()
    {
        println!("exception: stack range invalid");
        park();
    }
    // SP_EL1 is not an MRS/MSR-accessible register at EL1 (UNDEF).
    // While SPSel is still 1, SP is SP_EL1: save the thread pointer to
    // SP_EL0 (that register is legal at EL1), then `mov sp` to the
    // exception stack, then SPSel = 0.
    unsafe {
        core::arch::asm!(
            "msr vbar_el1, {v}",
            "isb",
            "mov {tmp}, sp",
            "msr sp_el0, {tmp}",
            "mov sp, {exc}",
            "msr spsel, #0",
            "isb",
            v = in(reg) vbar,
            exc = in(reg) exc_top,
            tmp = out(reg) _,
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
        if NEST_FATAL.swap(false, Ordering::SeqCst) {
            // Still on SP_EL1 / SPSel=1. Nested BRK takes the SP_ELx bank.
            unsafe {
                core::arch::asm!("brk #0");
            }
        }
        return;
    }
    uart::write_str_raw("exception: unhandled sync\n");
    let mut u = uart::raw();
    let _ = writeln!(u, "exception: unhandled sync esr={:#x} elr={:#x}", ctx.esr, ctx.elr);
    park();
}

#[no_mangle]
pub extern "C" fn handle_irq(_ctx: &mut ExceptionContext) {
    crate::gic::handle_irq();
}

#[no_mangle]
pub extern "C" fn handle_unhandled_exception(kind: u64) -> ! {
    uart::write_str_raw("exception: unhandled vector\n");
    let mut u = uart::raw();
    let _ = writeln!(u, "exception: unhandled vector {:#x}", kind);
    park();
}

#[no_mangle]
pub extern "C" fn handle_fatal_exception(esr: u64, elr: u64, kind: u64) -> ! {
    // Marker first, before any formatting, and without the UART mutex.
    uart::write_str_raw("exception: fatal nested\n");
    let mut u = uart::raw();
    let _ = writeln!(
        u,
        "exception: fatal esr={:#x} elr={:#x} kind={:#x}",
        esr, elr, kind
    );
    park();
}

#[no_mangle]
pub extern "C" fn fatal_probe_missed() -> ! {
    uart::write_str_raw("exception: fatal probe missed\n");
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

#[cfg(test)]
#[test_case]
fn spsel_uses_thread_stack() {
    assert_eq!(spsel(), 0);
    let sp = current_sp();
    assert!(sp > thread_stack_bottom());
    assert!(sp <= thread_stack_top());
}

#[cfg(test)]
#[test_case]
fn stacks_are_distinct_and_aligned() {
    let ranges = [
        (thread_stack_bottom(), thread_stack_top()),
        (exc_stack_bottom(), exc_stack_top()),
        (fatal_stack_bottom(), fatal_stack_top()),
    ];
    for (i, (lo, hi)) in ranges.iter().enumerate() {
        assert_eq!(lo & 0xf, 0, "stack {i} bottom misaligned");
        assert_eq!(hi & 0xf, 0, "stack {i} top misaligned");
        assert!(hi > lo, "stack {i} empty");
        for (j, (lo2, hi2)) in ranges.iter().enumerate() {
            if i == j {
                continue;
            }
            assert!(*hi <= *lo2 || *hi2 <= *lo, "stack {i} overlaps {j}");
        }
    }
}
