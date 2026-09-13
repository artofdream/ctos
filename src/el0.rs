//! EL0 first mile + user-TTBR0 read + standing context (P-SEC-3 / ADR-013).
//!
//! Deliberate `ERET` to EL0 on a one-page trampoline (UXN-clear, PXN).
//! `eret_to_el0` switches to a user TTBR0 that omits kernel `.data`/heap.
//! Four first-mile probes, then back to EL1:
//! - `SVC #0` returns via the lower-EL sync slot (`el0: svc`)
//! - `BR x0` to kernel `.data` must take a lower-EL IABORT (`el0: nx kernel`)
//! - `LDR` from kernel `.data` must take a lower-EL DABORT (`el0: no kernel read`)
//! - Standing dual-SVC on the user TTBR0 (`el0: standing` / `el0: restored`)
//!
//! Track A / A4 (ADR-024) promotes standing EL0 to the **supported path**
//! for a loaded freestanding image: `is_active()` is a real-task flag,
//! the A3 loader is how the payload appears, and the task runs until
//! `SYS_EXIT`. Unexpected lower-EL sync while a task is standing
//! restores fail-closed (`el0: restore-fail`) instead of parking.
//!
//! `is_active()` is true only while that standing context exists.
//! Lower-EL IRQ while standing is taken and returns to EL0 (ADR-040/041).
//! Lower-EL FIQ while standing is taken when GICC FIQEn is armed (ADR-043).
//! ADR-045 wires `eret_to_el0_serror` (A clear) + EXPECT for a standing
//! SError probe; QEMU 10 virt+`-cpu cortex-a57` has no working host inject
//! (`inject-nmi` → machine does not provide NMIs), so hello keeps
//! `el0: serror-park` when the taken path does not fire.
//! Default `ERET` to EL0 clears IRQ mask (ADR-041) and still masks A;
//! short non-standing trampoline probes stay masked.
//! PAN is typically unimplemented on `-cpu cortex-a57`. After
//! ADR-037/038 identity `.data`/heap tears, the standing/EL0 trampoline
//! path is `MSR TTBR0` + `ISB` only — no `TLBI VMALLE1` (ADR-039). ASID
//! isolation lives in `src/asid.rs`. TTBR1 private page + high-VA EL1
//! fetch live in `src/ttbr1.rs` (ADR-016 / ADR-017).
//! See el0.md. Not “EL0 isolated.”

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

use crate::exception;
use crate::gic;
use crate::frame;
use crate::loader;
use crate::paging;
use crate::timer;
use crate::uart;

/// AArch64 `SVC #0`.
const SVC0_A64: u32 = 0xD4000001;
/// AArch64 `SVC #1` (standing announce).
const SVC1_A64: u32 = 0xD4000021;
/// AArch64 `SVC #2` (standing restore).
const SVC2_A64: u32 = 0xD4000041;
/// AArch64 `MOVZ X1, #0x51A4` — user ran at EL0 after the first SVC.
const MOVZ_X1_MAGIC: u32 = 0xD28A3481;
/// AArch64 `BR X0`.
const BR_X0_A64: u32 = 0xD61F0000;
/// AArch64 `LDR X1, [X0]`.
const LDR_X1_X0_A64: u32 = 0xF9400001;
/// AArch64 `RET` — payload if the UXN probe failed and EL0 ran kernel data.
const RET_A64: u32 = 0xD65F03C0;

/// Kernel-image data the EL0 fetch must *not* execute (UXN on identity RAM).
#[used]
pub(crate) static mut KERNEL_DATA_BAIT: [u32; 2] = [RET_A64, RET_A64];

static ENTERED: AtomicBool = AtomicBool::new(false);
static NX_OK: AtomicBool = AtomicBool::new(false);
static READ_OK: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static USER_PC: AtomicU64 = AtomicU64::new(0);
static USER_SP: AtomicU64 = AtomicU64::new(0);
static USER_TTBR: AtomicU64 = AtomicU64::new(0);
/// 0 = none, 1 = ADR-013 probe, 2 = A4 standing task (ADR-024).
static KIND: AtomicU8 = AtomicU8::new(0);
static TASK_ACTIVE_SEEN: AtomicBool = AtomicBool::new(false);
static TASK_EXIT_SEEN: AtomicBool = AtomicBool::new(false);
static RESTORE_FAIL_SEEN: AtomicBool = AtomicBool::new(false);

const KIND_NONE: u8 = 0;
const KIND_PROBE: u8 = 1;
const KIND_TASK: u8 = 2;

/// Unused map-window page. The A4 fault probe `BR`s here (unmapped).
const TASK_FAULT_VA: u64 = paging::MAP_WINDOW + 8 * 4096;

/// True only while a standing user context is installed (ADR-013 / ADR-024).
#[allow(dead_code)] // hello build has no caller; `#[test_case]` + handler do.
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

/// True while a loaded **task** (not the dual-SVC probe) is standing.
#[allow(dead_code)]
pub fn is_task() -> bool {
    is_active() && KIND.load(Ordering::SeqCst) == KIND_TASK
}

/// Drop the standing flag and saved user PC/SP/TTBR0. Handler + teardown.
pub(crate) fn clear_active() {
    ACTIVE.store(false, Ordering::SeqCst);
    KIND.store(KIND_NONE, Ordering::SeqCst);
    USER_PC.store(0, Ordering::SeqCst);
    USER_SP.store(0, Ordering::SeqCst);
    USER_TTBR.store(0, Ordering::SeqCst);
}

/// Install a bounded standing **probe** context (ADR-013 / A1–A3 trips).
pub(crate) fn install_standing(pc: u64, sp: u64, ttbr: u64) {
    USER_PC.store(pc, Ordering::SeqCst);
    USER_SP.store(sp, Ordering::SeqCst);
    USER_TTBR.store(ttbr, Ordering::SeqCst);
    KIND.store(KIND_PROBE, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
}

/// Install a standing **task** (ADR-024). Payload comes from the A3 loader.
pub(crate) fn install_task(pc: u64, sp: u64, ttbr: u64) {
    USER_PC.store(pc, Ordering::SeqCst);
    USER_SP.store(sp, Ordering::SeqCst);
    USER_TTBR.store(ttbr, Ordering::SeqCst);
    KIND.store(KIND_TASK, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    uart::write_str_raw("el0: task-enter\n");
}

pub(crate) fn reset_task_flags() {
    TASK_ACTIVE_SEEN.store(false, Ordering::SeqCst);
    TASK_EXIT_SEEN.store(false, Ordering::SeqCst);
    RESTORE_FAIL_SEEN.store(false, Ordering::SeqCst);
}

/// `SYS_YIELD` / `SYS_UART_WRITE` saw `is_active()` on a standing task.
pub(crate) fn note_active_while_standing() {
    if !is_task() {
        return;
    }
    if !TASK_ACTIVE_SEEN.swap(true, Ordering::SeqCst) {
        uart::write_str_raw("el0: task-active\n");
    }
}

/// `SYS_EXIT` restore. Probe trips just clear; a task prints and records.
pub(crate) fn restore_exit() {
    if is_task() {
        TASK_EXIT_SEEN.store(true, Ordering::SeqCst);
        uart::write_str_raw("el0: task-exit\n");
        uart::write_str_raw("el0: task-restored\n");
    }
    clear_active();
}

/// Unexpected lower-EL sync while a task is standing: restore, do not park.
pub(crate) fn restore_fault() -> bool {
    if !is_task() {
        return false;
    }
    RESTORE_FAIL_SEEN.store(true, Ordering::SeqCst);
    clear_active();
    true
}

#[allow(dead_code)]
pub(crate) fn task_active_seen() -> bool {
    TASK_ACTIVE_SEEN.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn task_exit_seen() -> bool {
    TASK_EXIT_SEEN.load(Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn restore_fail_seen() -> bool {
    RESTORE_FAIL_SEEN.load(Ordering::SeqCst)
}

/// True after a successful SVC round-trip on this boot (first mile only).
#[allow(dead_code)]
pub fn entered_and_returned() -> bool {
    ENTERED.load(Ordering::SeqCst)
}

/// True after the lower-EL UXN / translation IABORT on kernel data was caught.
#[allow(dead_code)]
pub fn kernel_data_fetch_faulted() -> bool {
    NX_OK.load(Ordering::SeqCst)
}

/// True after the lower-EL DABORT on a kernel `.data` load was caught.
#[allow(dead_code)]
pub fn kernel_data_read_faulted() -> bool {
    READ_OK.load(Ordering::SeqCst)
}

fn sync_icache(ptr: *const u32) {
    let x = ptr as u64;
    unsafe {
        core::arch::asm!(
            "dc cvau, {x}",
            "dsb ish",
            "ic ivau, {x}",
            "dsb ish",
            "isb",
            x = in(reg) x,
            options(nostack, preserves_flags),
        );
    }
}

fn with_el0_page<F: FnOnce(*mut u32, u64) -> bool>(f: F) -> bool {
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let va = paging::EL0_PAGE;
    if !paging::map_el0_exec(va, pa) {
        frame::free(pa);
        return false;
    }
    let ptr = va as *mut u32;
    let ok = f(ptr, va);
    let _ = paging::unmap_page(va);
    frame::free(pa);
    ok
}

fn write_instr(ptr: *mut u32, insn: u32) {
    unsafe {
        core::ptr::write_volatile(ptr, insn);
        core::ptr::write_volatile(ptr.add(1), insn);
    }
    sync_icache(ptr);
}

fn write_standing(ptr: *mut u32) {
    unsafe {
        core::ptr::write_volatile(ptr, SVC1_A64);
        core::ptr::write_volatile(ptr.add(1), MOVZ_X1_MAGIC);
        core::ptr::write_volatile(ptr.add(2), SVC2_A64);
    }
    sync_icache(ptr);
    sync_icache(unsafe { ptr.add(1) });
    sync_icache(unsafe { ptr.add(2) });
}

/// AArch64 `WFI` — wait for the lower-EL IRQ probe (ADR-040).
const WFI_A64: u32 = 0xD503207F;

/// Standing dual-SVC with a `WFI` between announce and restore so a
/// timer IRQ can be taken from EL0 and return (ADR-040).
fn write_irq_standing(ptr: *mut u32) {
    unsafe {
        core::ptr::write_volatile(ptr, SVC1_A64);
        core::ptr::write_volatile(ptr.add(1), WFI_A64);
        core::ptr::write_volatile(ptr.add(2), MOVZ_X1_MAGIC);
        core::ptr::write_volatile(ptr.add(3), SVC2_A64);
    }
    sync_icache(ptr);
    sync_icache(unsafe { ptr.add(1) });
    sync_icache(unsafe { ptr.add(2) });
    sync_icache(unsafe { ptr.add(3) });
}

/// Timer IRQ taken from standing EL0 via **default** `eret_to_el0`
/// (IRQ-unmasked SPSR), then dual-SVC restore (ADR-041; path from ADR-040).
fn irq_while_standing() -> bool {
    if is_active() || !paging::user_map_ready() {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_irq_standing(ptr);
        let user_sp = va + 4096;
        exception::arm_el0_standing();
        exception::arm_el0_irq();
        install_standing(va, user_sp, paging::user_ttbr0());
        if !is_active() {
            clear_active();
            return false;
        }
        timer::arm_soon();
        // ADR-041: prove the tick without eret_to_el0_irq_enabled.
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if is_active() {
            clear_active();
            return false;
        }
        exception::el0_irq_caught()
            && exception::el0_standing_caught()
            && exception::el0_restored_caught()
    })
}

/// Timer as **FIQ** from standing EL0 (ADR-043): temporarily set GICC FIQEn,
/// enter with F clear / I set, `WFI`, then restore Group-0 → IRQ.
fn fiq_while_standing() -> bool {
    if is_active() || !paging::user_map_ready() {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_irq_standing(ptr);
        let user_sp = va + 4096;
        exception::arm_el0_standing();
        exception::arm_el0_fiq();
        install_standing(va, user_sp, paging::user_ttbr0());
        if !is_active() {
            clear_active();
            return false;
        }
        gic::route_group0_as_fiq();
        timer::arm_soon();
        unsafe {
            exception::eret_to_el0_fiq(black_box(va), 0, user_sp);
        }
        gic::route_group0_as_irq();
        if is_active() {
            clear_active();
            return false;
        }
        exception::el0_fiq_caught()
            && exception::el0_standing_caught()
            && exception::el0_restored_caught()
    })
}

/// AArch64 `MOVZ X2, #0x4000` — short A-clear window for host inject (TCG).
const MOVZ_X2_4000: u32 = 0xD2880002;
/// AArch64 `SUBS X2, X2, #1`.
const SUBS_X2_1: u32 = 0xF1000442;
/// AArch64 `B.NE` to previous insn (spin back to SUBS).
const B_NE_BACK1: u32 = 0x54FFFFE1;

/// Standing dual-SVC with a bounded spin (not WFI) so a host SError inject
/// can land while A is clear. WFI would hang forever if inject fails because
/// SPSR masks I/F (ADR-045). Keep the spin short on TCG.
fn write_serror_standing(ptr: *mut u32) {
    unsafe {
        core::ptr::write_volatile(ptr, SVC1_A64);
        core::ptr::write_volatile(ptr.add(1), MOVZ_X2_4000);
        core::ptr::write_volatile(ptr.add(2), SUBS_X2_1);
        core::ptr::write_volatile(ptr.add(3), B_NE_BACK1);
        core::ptr::write_volatile(ptr.add(4), MOVZ_X1_MAGIC);
        core::ptr::write_volatile(ptr.add(5), SVC2_A64);
    }
    for i in 0..6 {
        sync_icache(unsafe { ptr.add(i) });
    }
}

/// Standing EL0 with A clear (ADR-045). Returns true only if lower-EL SError
/// was taken and the dual-SVC restored. Without a working host inject this
/// returns false and the hello path prints `el0: serror-park`.
fn serror_while_standing() -> bool {
    if is_active() || !paging::user_map_ready() {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_serror_standing(ptr);
        let user_sp = va + 4096;
        let mut w = uart::raw();
        // UART cue for host QMP before ERET (ADR-045 / ADR-044 H1).
        let _ = writeln!(w, "el0: serror-arm");
        exception::arm_el0_standing();
        exception::arm_el0_serror();
        install_standing(va, user_sp, paging::user_ttbr0());
        if !is_active() {
            clear_active();
            return false;
        }
        unsafe {
            exception::eret_to_el0_serror(black_box(va), 0, user_sp);
        }
        if is_active() {
            clear_active();
            return false;
        }
        exception::el0_serror_caught()
            && exception::el0_standing_caught()
            && exception::el0_restored_caught()
    })
}

fn svc_roundtrip() -> bool {
    with_el0_page(|ptr, va| {
        write_instr(ptr, SVC0_A64);
        let user_sp = va + 4096;
        exception::arm_el0_svc();
        // Non-standing: mask IRQ so a mid-probe tick does not park.
        unsafe {
            exception::eret_to_el0_masked(black_box(va), 0, user_sp);
        }
        exception::el0_svc_caught()
    })
}

fn kernel_data_iabort() -> bool {
    with_el0_page(|ptr, va| {
        write_instr(ptr, BR_X0_A64);
        // EL0 uses TTBR0 only — pass the identity VA of kernel .data.
        let bait = paging::identity_pa(core::ptr::addr_of!(KERNEL_DATA_BAIT) as u64);
        let user_sp = va + 4096;
        exception::arm_el0_iabort();
        unsafe {
            exception::eret_to_el0_masked(black_box(va), bait, user_sp);
        }
        exception::el0_iabort_caught()
    })
}

/// Dual-SVC standing user on the user TTBR0. Not a trampoline-only flag:
/// the first SVC stays at EL0; the user `MOVZ` must run before restore.
fn stand_and_restore() -> bool {
    if is_active() || !paging::user_map_ready() {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_standing(ptr);
        let user_sp = va + 4096;
        exception::arm_el0_standing();
        install_standing(va, user_sp, paging::user_ttbr0());
        if !is_active()
            || USER_PC.load(Ordering::SeqCst) != va
            || USER_SP.load(Ordering::SeqCst) != user_sp
            || USER_TTBR.load(Ordering::SeqCst) != paging::user_ttbr0()
        {
            clear_active();
            return false;
        }
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        if is_active() {
            clear_active();
            return false;
        }
        exception::el0_standing_caught() && exception::el0_restored_caught()
    })
}

fn kernel_data_read_fault() -> bool {
    if !paging::user_map_ready() {
        return false;
    }
    let bait = paging::identity_pa(core::ptr::addr_of!(KERNEL_DATA_BAIT) as u64);
    if paging::user_mapped(bait) {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_instr(ptr, LDR_X1_X0_A64);
        let user_sp = va + 4096;
        exception::arm_el0_dabort();
        unsafe {
            exception::eret_to_el0_masked(black_box(va), bait, user_sp);
        }
        exception::el0_dabort_caught()
    })
}

/// Serial proof: SVC, cannot execute kernel data, cannot read kernel data.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !svc_roundtrip() {
        return false;
    }
    ENTERED.store(true, Ordering::SeqCst);
    if !kernel_data_iabort() {
        return false;
    }
    NX_OK.store(true, Ordering::SeqCst);
    if !kernel_data_read_fault() {
        return false;
    }
    READ_OK.store(true, Ordering::SeqCst);
    if !stand_and_restore() {
        return false;
    }
    // ADR-041: taken lower-EL IRQ while standing via default ERET.
    if !irq_while_standing() {
        return false;
    }
    // ADR-043: taken lower-EL FIQ while standing (GICC FIQEn + F-clear).
    if !fiq_while_standing() {
        return false;
    }
    // ADR-045: A-clear standing window for host QMP/`inject-nmi`.
    // On QEMU 10 virt+cortex-a57 the inject fails (no TYPE_NMI) — keep park.
    let serror_taken = serror_while_standing();
    // ADR-039: entry/return/stay used MSR TTBR0 + ISB only. Require the
    // identity tears that made dropping VMALLE1 honest.
    if !paging::identity_data_ready() || !paging::identity_heap_ready() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "el0: irq-default");
    if serror_taken {
        // Handler already printed `el0: serror`. Do not also print park.
    } else {
        // Honesty: taken path Planned until a working host inject exists.
        let _ = writeln!(w, "el0: serror-park");
    }
    let _ = writeln!(w, "el0: no-vmalle1");
    let _ = writeln!(w, "el0: ok");
    true
}

/// A4 / ADR-024: loaded app as a standing task until exit, plus fail-closed
/// restore on an unexpected EL0 fault. Not isolation. Not app hosting.
fn fault_task_restores() -> bool {
    if is_active() || !paging::user_map_ready() {
        return false;
    }
    reset_task_flags();
    with_el0_page(|ptr, va| {
        write_instr(ptr, BR_X0_A64);
        let user_sp = va + 4096;
        install_task(va, user_sp, paging::user_ttbr0());
        unsafe {
            exception::eret_to_el0(black_box(va), TASK_FAULT_VA, user_sp);
        }
        !is_active() && restore_fail_seen()
    })
}

/// Serial proof: standing task via the A3 loader + fail-closed fault restore.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_standing_task() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !loader::run_hello_as_task() {
        return false;
    }
    if !fault_task_restores() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "el0: task-ok");
    true
}

#[cfg(test)]
#[test_case]
fn el0_is_not_active_at_rest() {
    assert!(
        !is_active(),
        "no standing EL0 context outside stand_and_restore"
    );
}

#[cfg(test)]
#[test_case]
fn standing_el0_enter_leave() {
    assert!(!is_active(), "must start inactive");
    assert!(
        stand_and_restore(),
        "standing user must SVC #1, run at EL0, SVC #2, then restore"
    );
    assert!(!is_active(), "teardown must clear is_active()");
}

#[cfg(test)]
#[test_case]
fn standing_task_runs_until_exit() {
    assert!(!is_active(), "must start inactive");
    assert!(
        loader::run_hello_as_task(),
        "loaded A3 hello must stand as a task until SYS_EXIT"
    );
    assert!(!is_active(), "exit must clear is_active()");
    assert!(!is_task());
    assert!(task_active_seen(), "yield/uart must see is_active() true");
    assert!(task_exit_seen(), "SYS_EXIT must restore the standing task");
}

#[cfg(test)]
#[test_case]
fn standing_task_fault_restores() {
    assert!(!is_active(), "must start inactive");
    assert!(
        fault_task_restores(),
        "unexpected EL0 fault must restore fail-closed"
    );
    assert!(!is_active(), "fault restore must clear is_active()");
    assert!(restore_fail_seen());
}

#[cfg(test)]
#[test_case]
fn el0_svc_roundtrip() {
    assert!(paging::mmu_enabled());
    assert!(svc_roundtrip(), "ERET to EL0 must return via the SVC stub");
}

#[cfg(test)]
#[test_case]
fn el0_cannot_execute_kernel_data() {
    assert!(paging::mmu_enabled());
    assert!(
        kernel_data_iabort(),
        "EL0 fetch of kernel .data must take a lower-EL IABORT"
    );
}

#[cfg(test)]
#[test_case]
fn el0_cannot_read_kernel_data() {
    assert!(paging::mmu_enabled());
    assert!(paging::user_map_ready());
    assert!(
        kernel_data_read_fault(),
        "EL0 load of kernel .data must take a lower-EL DABORT"
    );
}

#[cfg(test)]
#[test_case]
fn pan_unclaimed_on_cortex_a57() {
    // Isolation stays Planned. cortex-a57 is ARMv8.0; PAN is usually absent.
    // Do not treat a missing PAN feature as a Failed probe.
    let _ = paging::pan_implemented();
}

#[cfg(test)]
#[test_case]
fn lower_el_irq_while_standing() {
    assert!(!is_active(), "must start inactive");
    assert!(
        irq_while_standing(),
        "timer IRQ from standing EL0 via default eret_to_el0 must return"
    );
    assert!(!is_active(), "teardown must clear is_active()");
}

#[cfg(test)]
#[test_case]
fn lower_el_irq_default_eret() {
    // Same probe as lower_el_irq_while_standing: enters via default
    // eret_to_el0 (SPSR I-clear), not a dedicated irq_enabled helper.
    assert!(!is_active(), "must start inactive");
    assert!(
        irq_while_standing(),
        "ADR-041: default standing ERET must take a timer IRQ"
    );
    assert!(!is_active(), "teardown must clear is_active()");
}

#[cfg(test)]
#[test_case]
fn lower_el_fiq_while_standing() {
    assert!(!is_active(), "must start inactive");
    assert!(
        fiq_while_standing(),
        "ADR-043: timer as FIQ from standing EL0 must return"
    );
    assert!(!is_active(), "teardown must clear is_active()");
}

// ADR-045: no `#[test_case]` for taken SError — needs host QMP inject, and
// QEMU 10 virt+`-cpu cortex-a57` reports "machine does not provide NMIs".
// Smoke may attempt QMP; park marker stays the Verified honesty line.


#[cfg(test)]
#[test_case]
fn el0_entry_without_vmalle1() {
    assert!(
        paging::identity_data_ready() && paging::identity_heap_ready(),
        "ADR-039 requires identity .data + heap tears before dropping VMALLE1"
    );
    assert!(
        kernel_data_read_fault(),
        "EL0 must still DABORT on identity kernel .data without TLBI VMALLE1"
    );
    assert!(
        stand_and_restore(),
        "standing dual-SVC must work without TLBI VMALLE1 on the trampoline"
    );
}

