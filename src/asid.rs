//! ASID-tagged TLB isolation mile (P-SEC-3c / ADR-013).
//!
//! Dual TTBR0 (ASID 1 vs ASID 2) with `nG` probe pages. The switch is
//! `MSR TTBR0` + `ISB` — no `TLBI VMALLE1`. A stale ASID-1 entry used
//! under ASID 2 is **Failed** (`asid: stale`).
//!
//! This is not “EL0 isolated.” PAN stays unclaimed on `-cpu cortex-a57`.
//! Standing EL0 lives in `src/el0.rs`. After ADR-039 the EL0 trampoline
//! also switches without `TLBI VMALLE1` (identity `.data`/heap torn).

use core::fmt::Write;
use core::hint::black_box;

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

const MAGIC_A: u64 = 0xA51D_0001;
const MAGIC_B: u64 = 0xA51D_0002;

fn dcache_civac(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc civac, {x}",
            x = in(reg) va,
            options(nostack, preserves_flags),
        );
    }
}

fn dsb_isb() {
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

fn load_u64(va: u64) -> u64 {
    unsafe { core::ptr::read_volatile(black_box(va) as *const u64) }
}

fn try_load(va: u64) -> u64 {
    let mut got = 0u64;
    unsafe {
        core::arch::asm!(
            "ldr {got}, [{ptr}]",
            got = inout(reg) got,
            ptr = in(reg) va,
            options(nostack),
        );
    }
    got
}

fn write_identity(pa: u64, magic: u64) {
    unsafe {
        core::ptr::write_volatile(pa as *mut u64, magic);
    }
    dcache_civac(pa);
}

fn restore_kernel() {
    paging::switch_ttbr0_no_tlbi(paging::kernel_ttbr0());
}

fn cleanup(pa_a: u64, pa_b: u64) {
    restore_kernel();
    let _ = paging::unmap_page(paging::ASID_DUAL_VA);
    let _ = paging::unmap_page(paging::ASID_CONFLICT_VA);
    let _ = paging::unmap_asid_b_page(paging::ASID_DUAL_VA);
    paging::invalidate_va(paging::ASID_DUAL_VA);
    paging::invalidate_va(paging::ASID_CONFLICT_VA);
    frame::free(pa_a);
    frame::free(pa_b);
}

fn stale(got: u64) -> bool {
    if got != MAGIC_A {
        return false;
    }
    uart::write_str_raw("asid: stale\n");
    true
}

/// Dual ASID + conflict/stale-entry probe. No `TLBI VMALLE1` on the switch.
fn run_probe() -> bool {
    if !paging::mmu_enabled() {
        return false;
    }
    if !paging::prepare_asid_b_tables() {
        return false;
    }
    let Some(pa_a) = frame::alloc() else {
        return false;
    };
    let Some(pa_b) = frame::alloc() else {
        frame::free(pa_a);
        return false;
    };
    write_identity(pa_a, MAGIC_A);
    write_identity(pa_b, MAGIC_B);
    dsb_isb();

    if !paging::map_page_ng(paging::ASID_DUAL_VA, pa_a) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if !paging::map_page_ng(paging::ASID_CONFLICT_VA, pa_a) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if !paging::map_asid_b_ng(paging::ASID_DUAL_VA, pa_b) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if paging::asid_b_mapped(paging::ASID_CONFLICT_VA) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if !paging::window_is_ng(paging::ASID_DUAL_VA)
        || !paging::window_is_ng(paging::ASID_CONFLICT_VA)
    {
        cleanup(pa_a, pa_b);
        return false;
    }
    dcache_civac(paging::ASID_DUAL_VA);
    dcache_civac(paging::ASID_CONFLICT_VA);
    paging::invalidate_va(paging::ASID_DUAL_VA);
    paging::invalidate_va(paging::ASID_CONFLICT_VA);

    paging::switch_ttbr0_no_tlbi(paging::asid_a_ttbr0());
    let a_dual = load_u64(paging::ASID_DUAL_VA);
    let a_conflict = load_u64(paging::ASID_CONFLICT_VA);
    if a_dual != MAGIC_A || a_conflict != MAGIC_A {
        cleanup(pa_a, pa_b);
        return false;
    }

    // ASID-A entries are now in the TLB. Switch without a full invalidate.
    paging::switch_ttbr0_no_tlbi(paging::asid_b_ttbr0());
    let b_dual = load_u64(paging::ASID_DUAL_VA);
    if stale(b_dual) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if b_dual != MAGIC_B {
        cleanup(pa_a, pa_b);
        return false;
    }
    uart::write_str_raw("asid: dual\n");

    exception::arm_asid_conflict();
    let leaked = try_load(paging::ASID_CONFLICT_VA);
    let faulted = exception::asid_conflict_caught();
    if stale(leaked) {
        cleanup(pa_a, pa_b);
        return false;
    }
    if !faulted {
        cleanup(pa_a, pa_b);
        return false;
    }

    paging::switch_ttbr0_no_tlbi(paging::asid_a_ttbr0());
    if load_u64(paging::ASID_DUAL_VA) != MAGIC_A || load_u64(paging::ASID_CONFLICT_VA) != MAGIC_A {
        cleanup(pa_a, pa_b);
        return false;
    }

    cleanup(pa_a, pa_b);
    true
}

/// Serial proof: dual ASID without `VMALLE1` + conflict fault (not stale).
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !run_probe() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "asid: ok");
    true
}

#[cfg(test)]
#[test_case]
fn asid_isolation_without_vmalle1() {
    assert!(paging::mmu_enabled());
    assert!(
        run_probe(),
        "ASID 1 vs 2 must not share an nG TLB entry without TLBI VMALLE1"
    );
}
