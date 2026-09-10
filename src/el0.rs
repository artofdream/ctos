//! EL0 first mile (P-SEC-3 / ADR-013).
//!
//! Deliberate `ERET` to EL0 on a one-page trampoline (UXN-clear, PXN).
//! Two probes, then back to EL1:
//! - `SVC #0` returns via the lower-EL sync slot (`el0: svc`)
//! - `BR x0` to kernel `.data` must take a lower-EL permission IABORT (`el0: nx kernel`)
//!
//! `is_active()` stays **false**: this is not a standing userspace, not
//! isolation, not a separate TTBR/ASID, and not PAN. Shared TTBR0 still
//! lets EL0 *read* kernel pages. See `docs/framework/el0.md`.

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

/// AArch64 `SVC #0`.
const SVC0_A64: u32 = 0xD4000001;
/// AArch64 `BR X0`.
const BR_X0_A64: u32 = 0xD61F0000;
/// AArch64 `RET` — payload if the UXN probe failed and EL0 ran kernel data.
const RET_A64: u32 = 0xD65F03C0;

/// Kernel-image data the EL0 fetch must *not* execute (UXN on identity RAM).
#[used]
static mut KERNEL_DATA_BAIT: [u32; 2] = [RET_A64, RET_A64];

static ENTERED: AtomicBool = AtomicBool::new(false);
static NX_OK: AtomicBool = AtomicBool::new(false);

/// Always false until a later ADR keeps an EL0 context (isolation / userspace).
#[allow(dead_code)] // hello build has no caller; `#[test_case]` does.
pub fn is_active() -> bool {
    false
}

/// True after a successful SVC round-trip on this boot (first mile only).
#[allow(dead_code)]
pub fn entered_and_returned() -> bool {
    ENTERED.load(Ordering::SeqCst)
}

/// True after the lower-EL UXN IABORT on kernel data was caught.
#[allow(dead_code)]
pub fn kernel_data_fetch_faulted() -> bool {
    NX_OK.load(Ordering::SeqCst)
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

/// Serial proof: EL0 entered+returned via SVC, and cannot execute kernel data.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() {
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
    let mut w = uart::raw();
    let _ = writeln!(w, "el0: ok");
    true
}

#[cfg(test)]
#[test_case]
fn el0_is_not_active() {
    assert!(
        !is_active(),
        "EL0 must stay inactive: first mile is not isolation or userspace"
    );
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
        "EL0 fetch of kernel .data must take a permission IABORT"
    );
}
