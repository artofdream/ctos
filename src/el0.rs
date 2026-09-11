//! EL0 first mile + user-TTBR0 read + standing context (P-SEC-3 / ADR-013).
//!
//! Deliberate `ERET` to EL0 on a one-page trampoline (UXN-clear, PXN).
//! `eret_to_el0` switches to a user TTBR0 that omits kernel `.data`/heap.
//! Four probes, then back to EL1:
//! - `SVC #0` returns via the lower-EL sync slot (`el0: svc`)
//! - `BR x0` to kernel `.data` must take a lower-EL IABORT (`el0: nx kernel`)
//! - `LDR` from kernel `.data` must take a lower-EL DABORT (`el0: no kernel read`)
//! - Standing dual-SVC on the user TTBR0 (`el0: standing` / `el0: restored`)
//!
//! `is_active()` is true only while that standing context exists.
//! Lower-EL IRQ/FIQ/SError still park (not exercised). PAN is typically
//! unimplemented on `-cpu cortex-a57`. The EL0 trampoline still TLBI ALL
//! because kernel `.data` leaves are global. ASID isolation lives in
//! `src/asid.rs`. TTBR1 private page + high-VA EL1 fetch live in
//! `src/ttbr1.rs` (ADR-016 / ADR-017).
//! See el0.md. Not “EL0 isolated.”

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::exception;
use crate::frame;
use crate::paging;
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
static mut KERNEL_DATA_BAIT: [u32; 2] = [RET_A64, RET_A64];

static ENTERED: AtomicBool = AtomicBool::new(false);
static NX_OK: AtomicBool = AtomicBool::new(false);
static READ_OK: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static USER_PC: AtomicU64 = AtomicU64::new(0);
static USER_SP: AtomicU64 = AtomicU64::new(0);
static USER_TTBR: AtomicU64 = AtomicU64::new(0);

/// True only while a standing user context is installed (ADR-013).
#[allow(dead_code)] // hello build has no caller; `#[test_case]` + handler do.
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

/// Drop the standing flag and saved user PC/SP/TTBR0. Handler + teardown.
pub(crate) fn clear_active() {
    ACTIVE.store(false, Ordering::SeqCst);
    USER_PC.store(0, Ordering::SeqCst);
    USER_SP.store(0, Ordering::SeqCst);
    USER_TTBR.store(0, Ordering::SeqCst);
}

fn install_standing(pc: u64, sp: u64, ttbr: u64) {
    USER_PC.store(pc, Ordering::SeqCst);
    USER_SP.store(sp, Ordering::SeqCst);
    USER_TTBR.store(ttbr, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
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

fn svc_roundtrip() -> bool {
    with_el0_page(|ptr, va| {
        write_instr(ptr, SVC0_A64);
        let user_sp = va + 4096;
        exception::arm_el0_svc();
        unsafe {
            exception::eret_to_el0(black_box(va), 0, user_sp);
        }
        exception::el0_svc_caught()
    })
}

fn kernel_data_iabort() -> bool {
    with_el0_page(|ptr, va| {
        write_instr(ptr, BR_X0_A64);
        let bait = core::ptr::addr_of!(KERNEL_DATA_BAIT) as u64;
        let user_sp = va + 4096;
        exception::arm_el0_iabort();
        unsafe {
            exception::eret_to_el0(black_box(va), bait, user_sp);
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
    let bait = core::ptr::addr_of!(KERNEL_DATA_BAIT) as u64;
    if paging::user_mapped(bait) {
        return false;
    }
    with_el0_page(|ptr, va| {
        write_instr(ptr, LDR_X1_X0_A64);
        let user_sp = va + 4096;
        exception::arm_el0_dabort();
        unsafe {
            exception::eret_to_el0(black_box(va), bait, user_sp);
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
    let mut w = uart::raw();
    let _ = writeln!(w, "el0: ok");
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
