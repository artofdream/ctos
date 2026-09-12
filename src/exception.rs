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
//! M7/ADR-012 identity-maps virt RAM with heap/frames PXN. [ADR-014]
//! unmaps a 4 KiB hole under each linker stack. [ADR-015] maps
//! `.text`/`.rodata` RO+X and `.data`/`.bss`/live linker stacks RW+NX.
//! Dedicated stacks + the nested `BRK` probe remain (ADR-005).
//! Execute-from-heap / execute-from-`.data` / write-to-RO-text are
//! caught here. Lower-EL AArch64 sync is live for the EL0 first mile,
//! the user-TTBR0 read mile, the standing dual-SVC (ADR-013), the
//! public SVC ABI (ADR-021), the libctos CRT payload (ADR-022), the
//! guest ELF PT_LOAD loader (ADR-023), standing EL0 as a task
//! until `SYS_EXIT` (ADR-024), thin VFS + memfs SVCs 19–23
//! (ADR-027), the
//! TTBR1 private-page DABORT (ADR-016), EL1 fetch from the TTBR1
//! RAM alias (ADR-017), and the ADR-018 identity-tear IABORT / EL0
//! DABORT: SVC, IABORT, DABORT.
//! Other lower-EL slots still park. After paging::init, `VBAR_EL1` is
//! the high alias of this table. Identity `_start` stays at `0x4008_0000`.
//! A dedicated identity text range plus live identity `.text` after the
//! boot stub are unmapped (ADR-019 / ADR-020). Identity `.rodata` is
//! unmapped after a pointer rewrite (ADR-025). Identity `.data` /
//! `.bss` / linker stacks are unmapped after SP relocate (ADR-037);
//! heap stays. PAN enable stays Planned on `-cpu cortex-a57` (ADR-026).
//! Not “the kernel moved.”

use core::arch::global_asm;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::println;
use crate::uart;

/// ESR_EL1.EC: BRK instruction in AArch64 state (ARM ARM).
const ESR_EC_BRK_A64: u64 = 0x3C;
/// Instruction Abort, same exception level (PXN / translation).
const ESR_EC_IABORT_CURRENT: u64 = 0x21;
/// Instruction Abort from lower EL (EL0 UXN / translation).
const ESR_EC_IABORT_LOWER: u64 = 0x20;
/// Data Abort, same exception level (guard-page translation / RO store).
const ESR_EC_DABORT_CURRENT: u64 = 0x25;
/// Data Abort from lower EL (EL0 read of kernel data).
const ESR_EC_DABORT_LOWER: u64 = 0x24;
/// SVC instruction from AArch64 (EL0 first mile).
const ESR_EC_SVC_A64: u64 = 0x15;
const ESR_IFSC_PERM_L1: u64 = 0x0D;
const ESR_IFSC_PERM_L2: u64 = 0x0E;
const ESR_IFSC_PERM_L3: u64 = 0x0F;
const ESR_DFSC_TRANS_L1: u64 = 0x05;
const ESR_DFSC_TRANS_L2: u64 = 0x06;
const ESR_DFSC_TRANS_L3: u64 = 0x07;
const ESR_DFSC_PERM_L1: u64 = 0x0D;
const ESR_DFSC_PERM_L2: u64 = 0x0E;
const ESR_DFSC_PERM_L3: u64 = 0x0F;
const ESR_IFSC_TRANS_L1: u64 = 0x05;
const ESR_IFSC_TRANS_L2: u64 = 0x06;
const ESR_IFSC_TRANS_L3: u64 = 0x07;
/// SPSR: DAIF masked, AArch64 EL1t (return from EL0 to SPSel=0).
const SPSR_EL1T_MASKED: u64 = 0x3C4;

static BRK_COUNT: AtomicU64 = AtomicU64::new(0);
static NEST_FATAL: AtomicBool = AtomicBool::new(false);
static EXPECT_NX: AtomicBool = AtomicBool::new(false);
static NX_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_GUARD: AtomicBool = AtomicBool::new(false);
static EXPECT_EL0_SVC: AtomicBool = AtomicBool::new(false);
static EL0_SVC_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_EL0_IABORT: AtomicBool = AtomicBool::new(false);
static EL0_IABORT_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_RO_NX: AtomicBool = AtomicBool::new(false);
static RO_NX_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_RO_WRITE: AtomicBool = AtomicBool::new(false);
static RO_WRITE_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_EL0_DABORT: AtomicBool = AtomicBool::new(false);
static EL0_DABORT_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_ASID_CONFLICT: AtomicBool = AtomicBool::new(false);
static ASID_CONFLICT_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_EL0_STANDING: AtomicBool = AtomicBool::new(false);
static EL0_STANDING_CAUGHT: AtomicBool = AtomicBool::new(false);
static EL0_RESTORED_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_TTBR1_DABORT: AtomicBool = AtomicBool::new(false);
static TTBR1_DABORT_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_IDENT_TEAR: AtomicBool = AtomicBool::new(false);
static IDENT_TEAR_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_IDENT_RODATA: AtomicBool = AtomicBool::new(false);
static IDENT_RODATA_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_IDENT_DATA: AtomicBool = AtomicBool::new(false);
static IDENT_DATA_CAUGHT: AtomicBool = AtomicBool::new(false);
static EXPECT_IDENT_EL0: AtomicBool = AtomicBool::new(false);
static IDENT_EL0_CAUGHT: AtomicBool = AtomicBool::new(false);
static EL0_CONT: AtomicU64 = AtomicU64::new(0);
static EL0_KSP: AtomicU64 = AtomicU64::new(0);

/// Standing user payload writes this into X1 between SVC #1 and SVC #2.
pub const STANDING_MAGIC: u64 = 0x51A4;
/// AArch64 `SVC #1` (standing announce).
const SVC1_IMM: u64 = 1;
/// AArch64 `SVC #2` (standing restore).
const SVC2_IMM: u64 = 2;

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
    static __stack_guard: u8;
    static __stack_bottom: u8;
    static __stack_top: u8;
    static __exc_stack_guard: u8;
    static __exc_stack_bottom: u8;
    static __exc_stack_top: u8;
    static __fatal_stack_guard: u8;
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

    // Lower EL, AArch64 — sync is live for the EL0 first mile (SVC / IABORT)
    .align 7
    b sync_lower_el
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

    // Lower EL AArch64 sync: same frame as current-EL; taken on SP_EL1.
    // Restore kernel TTBR0 before any .data/.bss access (user map omits them).
    sync_lower_el:
    sub sp, sp, #272
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    mrs x2, tpidr_el1
    cbz x2, 1f
    msr ttbr0_el1, x2
    isb
    tlbi vmalle1
    dsb ish
    isb
1:
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
    bl handle_sync_lower_el
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
    adrp x3, __fatal_stack_top
    add x3, x3, :lo12:__fatal_stack_top
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
    adrp x0, __stack_bottom
    add x0, x0, :lo12:__stack_bottom
    add x0, x0, #64
    mov sp, x0
    brk #0
    adrp x0, __stack_top
    add x0, x0, :lo12:__stack_top
    mov sp, x0
    b fatal_probe_missed
    "#
);

/// Link / identity address of a linker symbol.
///
/// `addr_of!` is ADRP from the current PC. After ADR-017 the handler
/// may run at the TTBR1 alias; mask back to the 39-bit identity VA so
/// guard / stack compares match `FAR_EL1` (the store used the low VA).
fn linker_sym(sym: *const u8) -> u64 {
    (sym as usize as u64) & ((1u64 << 39) - 1)
}

/// Live stack VAs are high after ADR-037. Guard holes stay identity
/// (FAR_EL1 on an identity store).
fn stack_va(ident: u64) -> u64 {
    if crate::paging::identity_data_ready() {
        crate::paging::to_high_va(ident)
    } else {
        ident
    }
}

pub fn thread_stack_guard() -> u64 {
    linker_sym(core::ptr::addr_of!(__stack_guard))
}

pub fn thread_stack_bottom() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__stack_bottom)))
}

pub fn thread_stack_top() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__stack_top)))
}

pub fn exc_stack_guard() -> u64 {
    linker_sym(core::ptr::addr_of!(__exc_stack_guard))
}

pub fn exc_stack_bottom() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__exc_stack_bottom)))
}

pub fn exc_stack_top() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__exc_stack_top)))
}

pub fn fatal_stack_guard() -> u64 {
    linker_sym(core::ptr::addr_of!(__fatal_stack_guard))
}

pub fn fatal_stack_bottom() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__fatal_stack_bottom)))
}

/// 4 KiB holes punched under each linker stack (ADR-014).
pub fn stack_guards() -> [u64; 3] {
    [
        thread_stack_guard(),
        exc_stack_guard(),
        fatal_stack_guard(),
    ]
}

fn far_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, far_el1", v = out(reg) v);
    }
    v
}

fn va_in_guard(va: u64) -> bool {
    let page = va & !0xfff;
    stack_guards().iter().any(|g| *g == page)
}

pub fn fatal_stack_top() -> u64 {
    stack_va(linker_sym(core::ptr::addr_of!(__fatal_stack_top)))
}

pub fn vector_table_addr() -> u64 {
    (exception_vectors as *const () as usize as u64) & ((1u64 << 39) - 1)
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

/// Program `VBAR_EL1`. Used at boot (identity) and after MMU (TTBR1 alias).
pub fn install_vbar(vbar: u64) -> bool {
    if vbar & 0x7ff != 0 {
        return false;
    }
    unsafe {
        core::arch::asm!(
            "msr vbar_el1, {v}",
            "isb",
            v = in(reg) vbar,
            options(nostack, preserves_flags),
        );
    }
    vbar_el1() == vbar
}

#[allow(dead_code)] // asserted in `#[test_case]`.
pub fn brk_count() -> u64 {
    BRK_COUNT.load(Ordering::SeqCst)
}

/// Arm the W^X probe: the next current-EL permission IABORT resumes at LR.
pub fn arm_nx_probe() {
    NX_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_NX.store(true, Ordering::SeqCst);
}

/// True if a permission IABORT was taken while the W^X probe was armed.
pub fn nx_probe_caught() -> bool {
    EXPECT_NX.store(false, Ordering::SeqCst);
    NX_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the ADR-014 probe: the next translation DABORT on a guard resumes at LR.
pub fn arm_guard_probe() {
    EXPECT_GUARD.store(true, Ordering::SeqCst);
}

pub fn disarm_guard_probe() {
    EXPECT_GUARD.store(false, Ordering::SeqCst);
}

/// Arm the EL0 SVC round-trip. Continuation / SP_EL0 are stored in `eret_to_el0`.
pub fn arm_el0_svc() {
    EL0_SVC_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_EL0_SVC.store(true, Ordering::SeqCst);
}

pub fn el0_svc_caught() -> bool {
    EXPECT_EL0_SVC.store(false, Ordering::SeqCst);
    EL0_SVC_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the EL0 UXN / kernel-data fetch probe.
pub fn arm_el0_iabort() {
    EL0_IABORT_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_EL0_IABORT.store(true, Ordering::SeqCst);
}

pub fn el0_iabort_caught() -> bool {
    EXPECT_EL0_IABORT.store(false, Ordering::SeqCst);
    EL0_IABORT_CAUGHT.load(Ordering::SeqCst)
}

/// Arm execute-from-`.data` (ADR-015): current-EL permission IABORT resumes at LR.
pub fn arm_ro_nx_probe() {
    RO_NX_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_RO_NX.store(true, Ordering::SeqCst);
}

pub fn ro_nx_probe_caught() -> bool {
    EXPECT_RO_NX.store(false, Ordering::SeqCst);
    RO_NX_CAUGHT.load(Ordering::SeqCst)
}

/// Arm write-to-RO-text (ADR-015): skip the faulting store.
pub fn arm_ro_write_probe() {
    RO_WRITE_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_RO_WRITE.store(true, Ordering::SeqCst);
}

pub fn ro_write_probe_caught() -> bool {
    EXPECT_RO_WRITE.store(false, Ordering::SeqCst);
    RO_WRITE_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the EL0 read-of-kernel-data probe (user TTBR0 / AP).
pub fn arm_el0_dabort() {
    EL0_DABORT_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_EL0_DABORT.store(true, Ordering::SeqCst);
}

pub fn el0_dabort_caught() -> bool {
    EXPECT_EL0_DABORT.store(false, Ordering::SeqCst);
    EL0_DABORT_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the ASID conflict probe: translation DABORT at `ASID_CONFLICT_VA`.
pub fn arm_asid_conflict() {
    ASID_CONFLICT_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_ASID_CONFLICT.store(true, Ordering::SeqCst);
}

pub fn asid_conflict_caught() -> bool {
    EXPECT_ASID_CONFLICT.store(false, Ordering::SeqCst);
    ASID_CONFLICT_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the standing EL0 dual-SVC (SVC #1 stay / SVC #2 restore).
pub fn arm_el0_standing() {
    EL0_STANDING_CAUGHT.store(false, Ordering::SeqCst);
    EL0_RESTORED_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_EL0_STANDING.store(true, Ordering::SeqCst);
}

pub fn el0_standing_caught() -> bool {
    EL0_STANDING_CAUGHT.load(Ordering::SeqCst)
}

pub fn el0_restored_caught() -> bool {
    EXPECT_EL0_STANDING.store(false, Ordering::SeqCst);
    EL0_RESTORED_CAUGHT.load(Ordering::SeqCst)
}

/// Arm the TTBR1 private-page EL0 load (ADR-016).
pub fn arm_ttbr1_dabort() {
    TTBR1_DABORT_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_TTBR1_DABORT.store(true, Ordering::SeqCst);
}

pub fn ttbr1_dabort_caught() -> bool {
    EXPECT_TTBR1_DABORT.store(false, Ordering::SeqCst);
    TTBR1_DABORT_CAUGHT.load(Ordering::SeqCst)
}

/// Arm EL1 fetch of a torn identity text VA (ADR-018 / ADR-019).
pub fn arm_ident_tear() {
    IDENT_TEAR_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_IDENT_TEAR.store(true, Ordering::SeqCst);
}

pub fn ident_tear_caught() -> bool {
    EXPECT_IDENT_TEAR.store(false, Ordering::SeqCst);
    IDENT_TEAR_CAUGHT.load(Ordering::SeqCst)
}

/// Arm EL1 load of a torn identity `.rodata` VA (ADR-025).
pub fn arm_ident_rodata() {
    IDENT_RODATA_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_IDENT_RODATA.store(true, Ordering::SeqCst);
}

pub fn ident_rodata_caught() -> bool {
    EXPECT_IDENT_RODATA.store(false, Ordering::SeqCst);
    IDENT_RODATA_CAUGHT.load(Ordering::SeqCst)
}

/// Arm EL1 load of a torn identity `.data` VA (ADR-037).
pub fn arm_ident_data() {
    IDENT_DATA_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_IDENT_DATA.store(true, Ordering::SeqCst);
}

pub fn ident_data_caught() -> bool {
    EXPECT_IDENT_DATA.store(false, Ordering::SeqCst);
    IDENT_DATA_CAUGHT.load(Ordering::SeqCst)
}

/// Arm EL0 load of a torn identity text VA (ADR-018 / ADR-019).
pub fn arm_ident_el0() {
    IDENT_EL0_CAUGHT.store(false, Ordering::SeqCst);
    EXPECT_IDENT_EL0.store(true, Ordering::SeqCst);
}

pub fn ident_el0_caught() -> bool {
    EXPECT_IDENT_EL0.store(false, Ordering::SeqCst);
    IDENT_EL0_CAUGHT.load(Ordering::SeqCst)
}

/// `ERET` to EL0 at `user_pc` with `x0 = user_arg` and `SP_EL0 = user_sp`.
/// Returns after the lower-EL handler sends us back to EL1t.
///
/// Caller must arm `arm_el0_svc` / `arm_el0_iabort` first. Callee-saved
/// GPRs are saved on the kernel thread stack across the trip.
#[allow(dead_code)] // hello + `#[test_case]` via `src/el0.rs`.
pub unsafe fn eret_to_el0(user_pc: u64, user_arg: u64, user_sp: u64) {
    // After ADR-037 identity `.bss` is unmapped — store via the high twin.
    let kslot = crate::paging::to_high_va(crate::paging::identity_pa(EL0_KSP.as_ptr() as u64));
    let cslot = crate::paging::to_high_va(crate::paging::identity_pa(EL0_CONT.as_ptr() as u64));
    core::arch::asm!(
        "stp x19, x20, [sp, #-16]!",
        "stp x21, x22, [sp, #-16]!",
        "stp x23, x24, [sp, #-16]!",
        "stp x25, x26, [sp, #-16]!",
        "stp x27, x28, [sp, #-16]!",
        "stp x29, x30, [sp, #-16]!",
        "mov {ksp}, sp",
        "adr {cont}, 2f",
        "str {ksp}, [{kslot}]",
        "str {cont}, [{cslot}]",
        "dsb sy",
        // SPSel=0 makes `msr SP_EL0` UNDEF (current SP *is* SP_EL0).
        // Switch to SP_EL1 so the user SP_EL0 write is legal, then ERET.
        "msr spsel, #1",
        "msr elr_el1, {upc}",
        "msr spsr_el1, {spsr}",
        "msr sp_el0, {usp}",
        "mov x0, {uarg}",
        "msr ttbr0_el1, {uttbr}",
        "isb",
        "tlbi vmalle1",
        "dsb ish",
        "isb",
        "eret",
        "2:",
        "ldp x29, x30, [sp], #16",
        "ldp x27, x28, [sp], #16",
        "ldp x25, x26, [sp], #16",
        "ldp x23, x24, [sp], #16",
        "ldp x21, x22, [sp], #16",
        "ldp x19, x20, [sp], #16",
        ksp = out(reg) _,
        cont = out(reg) _,
        kslot = in(reg) kslot,
        cslot = in(reg) cslot,
        upc = in(reg) user_pc,
        spsr = in(reg) 0x3c0u64,
        usp = in(reg) user_sp,
        uarg = in(reg) user_arg,
        uttbr = in(reg) crate::paging::user_ttbr0(),
        lateout("x0") _,
        lateout("x1") _,
        lateout("x2") _,
        lateout("x3") _,
        lateout("x4") _,
        lateout("x5") _,
        lateout("x6") _,
        lateout("x7") _,
        lateout("x8") _,
        lateout("x9") _,
        lateout("x10") _,
        lateout("x11") _,
        lateout("x12") _,
        lateout("x13") _,
        lateout("x14") _,
        lateout("x15") _,
        lateout("x16") _,
        lateout("x17") _,
        lateout("x18") _,
        options(preserves_flags),
    );
}

fn return_from_el0(ctx: &mut ExceptionContext) {
    let ksp = EL0_KSP.load(Ordering::SeqCst);
    unsafe {
        core::arch::asm!(
            "msr sp_el0, {sp}",
            "isb",
            sp = in(reg) ksp,
            options(nostack, preserves_flags),
        );
    }
    ctx.elr = EL0_CONT.load(Ordering::SeqCst);
    ctx.spsr = SPSR_EL1T_MASKED;
}

fn svc_imm(esr: u64) -> u64 {
    esr & 0xffff
}

/// Stay at EL0 after a standing SVC: put user TTBR0 back.
/// AArch64 SVC preferred return is the *next* insn — do not add 4.
/// Last `.data` access must finish before the `msr` (user map omits it).
fn stay_at_el0(_ctx: &mut ExceptionContext) {
    let uttbr = crate::paging::user_ttbr0();
    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {t}",
            "isb",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            t = in(reg) uttbr,
            options(nostack, preserves_flags),
        );
    }
}

fn is_trans_iabort(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let ifsc = esr & 0x3f;
    ec == ESR_EC_IABORT_CURRENT
        && (ifsc == ESR_IFSC_TRANS_L1 || ifsc == ESR_IFSC_TRANS_L2 || ifsc == ESR_IFSC_TRANS_L3)
}

fn is_perm_iabort(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let ifsc = esr & 0x3f;
    ec == ESR_EC_IABORT_CURRENT
        && (ifsc == ESR_IFSC_PERM_L1 || ifsc == ESR_IFSC_PERM_L2 || ifsc == ESR_IFSC_PERM_L3)
}

fn is_perm_iabort_lower(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let ifsc = esr & 0x3f;
    ec == ESR_EC_IABORT_LOWER
        && (ifsc == ESR_IFSC_PERM_L1 || ifsc == ESR_IFSC_PERM_L2 || ifsc == ESR_IFSC_PERM_L3)
}

fn is_trans_iabort_lower(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let ifsc = esr & 0x3f;
    ec == ESR_EC_IABORT_LOWER
        && (ifsc == ESR_IFSC_TRANS_L1 || ifsc == ESR_IFSC_TRANS_L2 || ifsc == ESR_IFSC_TRANS_L3)
}

fn is_el0_kernel_fetch(esr: u64) -> bool {
    is_perm_iabort_lower(esr) || is_trans_iabort_lower(esr)
}

fn is_trans_dabort(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let dfsc = esr & 0x3f;
    ec == ESR_EC_DABORT_CURRENT
        && (dfsc == ESR_DFSC_TRANS_L1 || dfsc == ESR_DFSC_TRANS_L2 || dfsc == ESR_DFSC_TRANS_L3)
}

fn is_perm_dabort(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let dfsc = esr & 0x3f;
    ec == ESR_EC_DABORT_CURRENT
        && (dfsc == ESR_DFSC_PERM_L1 || dfsc == ESR_DFSC_PERM_L2 || dfsc == ESR_DFSC_PERM_L3)
}

fn is_el0_kernel_read(esr: u64) -> bool {
    let ec = (esr >> 26) & 0x3f;
    let dfsc = esr & 0x3f;
    if ec != ESR_EC_DABORT_LOWER {
        return false;
    }
    dfsc == ESR_DFSC_TRANS_L1
        || dfsc == ESR_DFSC_TRANS_L2
        || dfsc == ESR_DFSC_TRANS_L3
        || dfsc == ESR_DFSC_PERM_L1
        || dfsc == ESR_DFSC_PERM_L2
        || dfsc == ESR_DFSC_PERM_L3
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
    if is_perm_iabort(ctx.esr) && EXPECT_NX.swap(false, Ordering::SeqCst) {
        NX_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("wx: nx heap\n");
        // Resume as if the `blr` to the NX payload returned.
        ctx.elr = ctx.lr;
        return;
    }
    if is_perm_iabort(ctx.esr) && EXPECT_RO_NX.swap(false, Ordering::SeqCst) {
        RO_NX_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ro: nx data\n");
        ctx.elr = ctx.lr;
        return;
    }
    if is_perm_dabort(ctx.esr) && EXPECT_RO_WRITE.swap(false, Ordering::SeqCst) {
        RO_WRITE_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ro: write fault\n");
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
    if is_trans_dabort(ctx.esr) && EXPECT_GUARD.load(Ordering::SeqCst) && va_in_guard(far_el1()) {
        crate::guard::note_fault();
        // Skip the faulting store. Unlike the heap NX `blr`, LR is the
        // caller — jumping there would abandon the callee stack frame.
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
    if is_trans_dabort(ctx.esr)
        && EXPECT_ASID_CONFLICT.swap(false, Ordering::SeqCst)
        && (far_el1() & !0xfff) == crate::paging::ASID_CONFLICT_VA
    {
        ASID_CONFLICT_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("asid: conflict\n");
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
    if is_trans_iabort(ctx.esr)
        && EXPECT_IDENT_TEAR.swap(false, Ordering::SeqCst)
        && crate::paging::is_torn_identity_va(ctx.elr)
    {
        IDENT_TEAR_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ident: fault\n");
        ctx.elr = ctx.lr;
        return;
    }
    if is_trans_dabort(ctx.esr)
        && EXPECT_IDENT_RODATA.swap(false, Ordering::SeqCst)
        && crate::paging::is_torn_identity_va(far_el1())
    {
        IDENT_RODATA_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ident: rodata-fault\n");
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
    if is_trans_dabort(ctx.esr)
        && EXPECT_IDENT_DATA.swap(false, Ordering::SeqCst)
        && crate::paging::is_torn_identity_va(far_el1())
    {
        IDENT_DATA_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ident: data-fault\n");
        ctx.elr = ctx.elr.wrapping_add(4);
        return;
    }
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
pub extern "C" fn handle_sync_lower_el(ctx: &mut ExceptionContext) {
    let ec = (ctx.esr >> 26) & 0x3f;
    if ec == ESR_EC_SVC_A64
        && svc_imm(ctx.esr) == crate::syscall::SVC_PROBE_RETURN
        && EXPECT_EL0_SVC.swap(false, Ordering::SeqCst)
    {
        EL0_SVC_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: svc\n");
        return_from_el0(ctx);
        return;
    }
    if ec == ESR_EC_SVC_A64
        && svc_imm(ctx.esr) == SVC1_IMM
        && EXPECT_EL0_STANDING.load(Ordering::SeqCst)
        && crate::el0::is_active()
    {
        EL0_STANDING_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: standing\n");
        stay_at_el0(ctx);
        return;
    }
    if ec == ESR_EC_SVC_A64
        && svc_imm(ctx.esr) == SVC2_IMM
        && EXPECT_EL0_STANDING.swap(false, Ordering::SeqCst)
        && EL0_STANDING_CAUGHT.load(Ordering::SeqCst)
        && ctx.x[1] == STANDING_MAGIC
    {
        crate::el0::clear_active();
        EL0_RESTORED_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: restored\n");
        return_from_el0(ctx);
        return;
    }
    if ec == ESR_EC_SVC_A64 {
        if let Some(action) = crate::syscall::dispatch(ctx) {
            match action {
                crate::syscall::SvcAction::StayEl0 => stay_at_el0(ctx),
                crate::syscall::SvcAction::ReturnEl1 => {
                    crate::el0::restore_exit();
                    return_from_el0(ctx);
                }
            }
            return;
        }
    }
    if is_el0_kernel_fetch(ctx.esr) && EXPECT_EL0_IABORT.swap(false, Ordering::SeqCst) {
        EL0_IABORT_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: nx kernel\n");
        return_from_el0(ctx);
        return;
    }
    if is_el0_kernel_read(ctx.esr) && EXPECT_EL0_DABORT.swap(false, Ordering::SeqCst) {
        EL0_DABORT_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: no kernel read\n");
        return_from_el0(ctx);
        return;
    }
    if is_el0_kernel_read(ctx.esr)
        && EXPECT_TTBR1_DABORT.swap(false, Ordering::SeqCst)
        && (far_el1() & !0xfff) == crate::paging::TTBR1_PRIV
    {
        TTBR1_DABORT_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ttbr1: no el0\n");
        return_from_el0(ctx);
        return;
    }
    if is_el0_kernel_read(ctx.esr)
        && EXPECT_IDENT_EL0.swap(false, Ordering::SeqCst)
        && crate::paging::is_torn_identity_va(far_el1())
    {
        IDENT_EL0_CAUGHT.store(true, Ordering::SeqCst);
        uart::write_str_raw("ident: no el0\n");
        return_from_el0(ctx);
        return;
    }
    if crate::el0::restore_fault() {
        uart::write_str_raw("el0: restore-fail\n");
        return_from_el0(ctx);
        return;
    }
    uart::write_str_raw("exception: unhandled lower sync\n");
    let mut u = uart::raw();
    let _ = writeln!(
        u,
        "exception: unhandled lower sync esr={:#x} elr={:#x}",
        ctx.esr, ctx.elr
    );
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
    // After paging::init the table is fetched via the TTBR1 RAM alias.
    // The link address stays identity (`0x4008_0000` + offset).
    let ident = vector_table_addr();
    let high = crate::paging::to_high_va(ident);
    assert_eq!(vbar_el1(), high);
    assert!(crate::paging::is_high_va(vbar_el1()));
    assert_eq!(high.wrapping_sub(ident), crate::paging::TTBR1_BASE);
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
