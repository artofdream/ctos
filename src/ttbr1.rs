//! TTBR1 kernel-private page (P-SEC-3e / ADR-016 first cut).
//!
//! Enables a high-half walk (`TCR.EPD1` clear, T1SZ=25) and maps one
//! EL1-only page at `paging::TTBR1_PRIV`. EL1 can read it; EL0 must
//! take a lower-EL DABORT (`ttbr1: no el0`). The kernel still runs
//! from the identity TTBR0 map — full higher-half teardown is Planned.
//!
//! Not “EL0 isolated.” PAN stays unclaimed on `-cpu cortex-a57`.

use core::fmt::Write;
use core::hint::black_box;

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

const MAGIC: u64 = 0x771B_1001;
/// AArch64 `LDR X1, [X0]`.
const LDR_X1_X0_A64: u32 = 0xF9400001;

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

fn load_u64(va: u64) -> u64 {
    unsafe { core::ptr::read_volatile(black_box(va) as *const u64) }
}

fn write_u64(va: u64, v: u64) {
    unsafe {
        core::ptr::write_volatile(black_box(va) as *mut u64, v);
    }
    dcache_civac(va);
}

fn cleanup(pa: u64) {
    let _ = paging::unmap_ttbr1_priv();
    frame::free(pa);
}

fn el0_load_high() -> bool {
    let Some(code_pa) = frame::alloc() else {
        return false;
    };
    let va = paging::EL0_PAGE;
    if !paging::map_el0_exec(va, code_pa) {
        frame::free(code_pa);
        return false;
    }
    let ptr = va as *mut u32;
    unsafe {
        core::ptr::write_volatile(ptr, LDR_X1_X0_A64);
        core::ptr::write_volatile(ptr.add(1), LDR_X1_X0_A64);
    }
    sync_icache(ptr);
    let user_sp = va + 4096;
    exception::arm_ttbr1_dabort();
    unsafe {
        exception::eret_to_el0(black_box(va), paging::TTBR1_PRIV, user_sp);
    }
    let ok = exception::ttbr1_dabort_caught();
    let _ = paging::unmap_page(va);
    frame::free(code_pa);
    ok
}

/// EL1 can use the high page; EL0 cannot. Identity teardown stays Planned.
fn run_probe() -> bool {
    if !paging::mmu_enabled() || !paging::ttbr1_ready() || !paging::ttbr1_walks_enabled() {
        return false;
    }
    if !paging::user_map_ready() {
        return false;
    }
    let Some(pa) = frame::alloc() else {
        return false;
    };
    write_u64(pa, 0);
    dsb_isb();
    if !paging::map_ttbr1_priv(pa) {
        frame::free(pa);
        return false;
    }
    if !paging::high_mapped(paging::TTBR1_PRIV) {
        cleanup(pa);
        return false;
    }
    if paging::user_mapped(paging::TTBR1_PRIV) {
        cleanup(pa);
        return false;
    }
    write_u64(paging::TTBR1_PRIV, MAGIC);
    dcache_civac(pa);
    dsb_isb();
    if load_u64(paging::TTBR1_PRIV) != MAGIC {
        cleanup(pa);
        return false;
    }
    if load_u64(pa) != MAGIC {
        cleanup(pa);
        return false;
    }
    uart::write_str_raw("ttbr1: el1\n");

    if !el0_load_high() {
        cleanup(pa);
        return false;
    }

    cleanup(pa);
    if paging::high_mapped(paging::TTBR1_PRIV) {
        return false;
    }
    true
}

/// Serial proof: EL1 high access + EL0 DABORT on the private page.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !run_probe() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "ttbr1: ok");
    true
}

#[cfg(test)]
#[test_case]
fn ttbr1_el1_sees_priv_el0_does_not() {
    assert!(paging::mmu_enabled());
    assert!(paging::ttbr1_walks_enabled());
    assert!(
        run_probe(),
        "EL1 must use TTBR1_PRIV; EL0 must take a lower-EL DABORT"
    );
}
