//! TTBR1 private page (ADR-016) + EL1 fetch from the high RAM alias (ADR-017).
//!
//! ADR-016: one EL1-only page at `paging::TTBR1_PRIV`. EL1 can read it;
//! EL0 must take a lower-EL DABORT (`ttbr1: no el0`).
//!
//! ADR-017: identity RAM is aliased at `va + TTBR1_BASE`. A real EL1
//! function is invoked through that high VA and prints `ttbr1: el1 exec`
//! from the fetched path. `VBAR_EL1` is the high alias of the vector
//! table (`ttbr1: vbar`). `_start` / QEMU `-kernel` stay at `0x4008_0000`.
//! ADR-018 clones RAM tables and unmaps one identity text page; this
//! module is still the exec / private-page mile. Full teardown Planned.
//!
//! Not “EL0 isolated.” PAN stays unclaimed on `-cpu cortex-a57`.

use core::fmt::Write;
use core::hint::black_box;
use core::mem::transmute;

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
    // Non-standing short probe: keep IRQ masked (ADR-041).
    unsafe {
        exception::eret_to_el0_masked(black_box(va), paging::TTBR1_PRIV, user_sp);
    }
    let ok = exception::ttbr1_dabort_caught();
    let _ = paging::unmap_page(va);
    frame::free(code_pa);
    ok
}

/// Real EL1 path fetched via TTBR1. Must stay in `.text` (RO+X).
#[inline(never)]
#[no_mangle]
pub extern "C" fn ttbr1_high_el1_path() -> u64 {
    let pc: u64;
    unsafe {
        core::arch::asm!(
            "adr {p}, 1f",
            "1:",
            p = out(reg) pc,
            options(nomem, nostack, preserves_flags),
        );
    }
    // UART MMIO is an absolute identity address (`0x0900_0000`). The
    // string lives in `.rodata`, reached by ADRP from this high PC —
    // that resolves to the TTBR1 RAM alias (same PA as identity).
    uart::write_str_raw("ttbr1: el1 exec\n");
    pc
}

/// EL1 instruction fetch from the high RAM alias + high VBAR (ADR-017).
fn run_high_exec() -> bool {
    if !paging::mmu_enabled() || !paging::ttbr1_ready() || !paging::ttbr1_walks_enabled() {
        return false;
    }
    if !paging::high_alias_ready() {
        return false;
    }
    // After the ADR-019 high jump, `fn as usize` may be a high VA
    // (ADRP from the current PC). Page-table compares need identity.
    let ident = paging::identity_pa(ttbr1_high_el1_path as *const () as usize as u64);
    if ident < paging::KERNEL_TEXT || ident >= paging::data_start() {
        return false;
    }
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return false;
    }
    if !paging::high_is_executable(high) {
        return false;
    }
    let vbar = exception::vbar_el1();
    let vbar_ident = exception::vector_table_addr();
    if vbar != paging::to_high_va(vbar_ident) {
        return false;
    }
    let f: extern "C" fn() -> u64 = unsafe { transmute(black_box(high)) };
    let pc = f();
    if !paging::is_high_va(pc) {
        return false;
    }
    // `adr` is the first insn; allow a tiny prefix (nop/BTI).
    if pc < high || pc.wrapping_sub(high) > 32 {
        return false;
    }
    uart::write_str_raw("ttbr1: vbar\n");
    true
}

/// EL1 can use the high page; EL0 cannot. Identity teardown stays Planned.
fn run_priv_probe() -> bool {
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

/// Serial proof: high-VA EL1 fetch + EL1 private page + EL0 DABORT.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !run_high_exec() {
        uart::write_str_raw("ttbr1: exec missed\n");
        return false;
    }
    if !run_priv_probe() {
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
        run_priv_probe(),
        "EL1 must use TTBR1_PRIV; EL0 must take a lower-EL DABORT"
    );
}

#[cfg(test)]
#[test_case]
fn el1_executes_from_ttbr1_high_va() {
    assert!(paging::mmu_enabled());
    assert!(paging::high_alias_ready());
    assert!(
        run_high_exec(),
        "EL1 must fetch a real path from the TTBR1 RAM alias"
    );
}
