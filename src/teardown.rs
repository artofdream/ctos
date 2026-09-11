//! Identity-teardown cuts (ADR-018 + ADR-019).
//!
//! After MMU + high VBAR, the post-MMU continuation jumps to its
//! TTBR1 alias (`ident: jump`) and unmaps a dedicated 16 KiB identity
//! text range (`__ident_tear_*`, ADR-018 page 0 + ADR-019 rest).
//! High twins stay executable (`L2_HIGH_RAM` clone). Live `.text` /
//! `.rodata` / `.data` / heap stay identity-mapped: rustc `dyn Write`
//! vtables still `BLR` identity fn pointers, so a full `.text` yank
//! dies on the first `println!`.
//!
//! Probes: EL1 fetch of a torn identity VA faults (`ident: fault`);
//! EL1 fetch of the high twin still runs (`ident: high` / `ident: text`);
//! EL0 load of a torn VA faults (`ident: no el0`). `_start` / QEMU
//! `-kernel` stay at `0x4008_0000`. Not “the kernel moved.” Not
//! “EL0 isolated.” PAN unclaimed. Full teardown stays Planned.

use core::fmt::Write;
use core::hint::black_box;
use core::mem::transmute;

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

/// AArch64 `LDR X1, [X0]`.
const LDR_X1_X0_A64: u32 = 0xF9400001;

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

/// Lives only on the torn identity page. Invoked through the high VA.
#[link_section = "ident_tear"]
#[inline(never)]
#[no_mangle]
pub extern "C" fn ident_tear_el1_path() -> u64 {
    let pc: u64;
    unsafe {
        core::arch::asm!(
            "adr {p}, 1f",
            "1:",
            p = out(reg) pc,
            options(nomem, nostack, preserves_flags),
        );
    }
    uart::write_str_raw("ident: high\n");
    pc
}

/// Second page of the dedicated tear range (ADR-019). Not the
/// `0x4008_0000` boot stub and not live `.text` (fmt vtables).
#[link_section = "ident_tear2"]
#[inline(never)]
#[no_mangle]
pub extern "C" fn ident_range_el1_path() -> u64 {
    let pc: u64;
    unsafe {
        core::arch::asm!(
            "adr {p}, 1f",
            "1:",
            p = out(reg) pc,
            options(nomem, nostack, preserves_flags),
        );
    }
    uart::write_str_raw("ident: text\n");
    pc
}

fn ident_page_layout_ok() -> bool {
    let va = paging::ident_tear_page();
    if va & 0xfff != 0 || paging::ident_tear_end() < va + 4 * 4096 {
        return false;
    }
    if va <= paging::KERNEL_TEXT || va >= paging::data_start() {
        return false;
    }
    let fn_va = paging::identity_pa(ident_tear_el1_path as *const () as usize as u64);
    (fn_va & !0xfff) == va
}

fn text_fn_layout_ok() -> bool {
    let fn_va = paging::identity_pa(ident_range_el1_path as *const () as usize as u64);
    let page = fn_va & !0xfff;
    page != paging::ident_tear_page()
        && page >= paging::ident_tear_page()
        && page < paging::ident_tear_end()
        && paging::is_torn_identity_va(fn_va)
}

fn run_high_path(ident: u64, max_delta: u64) -> Option<u64> {
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return None;
    }
    if !paging::high_is_executable(high) {
        return None;
    }
    let f: extern "C" fn() -> u64 = unsafe { transmute(black_box(high)) };
    let pc = f();
    if paging::is_high_va(pc) && pc >= high && pc.wrapping_sub(high) <= max_delta {
        Some(pc)
    } else {
        None
    }
}

fn run_high_after_tear() -> bool {
    if !ident_page_layout_ok() {
        return false;
    }
    let ident = paging::identity_pa(ident_tear_el1_path as *const () as usize as u64);
    run_high_path(ident, 32).is_some()
}

fn run_ident_fault() -> bool {
    let ident = paging::identity_pa(ident_tear_el1_path as *const () as usize as u64);
    if paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_tear();
    let f: extern "C" fn() -> u64 = unsafe { transmute(black_box(ident)) };
    let _ = black_box(f)();
    exception::ident_tear_caught()
}

fn run_text_high() -> bool {
    if !text_fn_layout_ok() {
        return false;
    }
    let ident = paging::identity_pa(ident_range_el1_path as *const () as usize as u64);
    run_high_path(ident, 32).is_some()
}

fn run_text_fault() -> bool {
    let ident = paging::identity_pa(ident_range_el1_path as *const () as usize as u64);
    if paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_tear();
    let f: extern "C" fn() -> u64 = unsafe { transmute(black_box(ident)) };
    let _ = black_box(f)();
    exception::ident_tear_caught()
}

fn el0_load_torn() -> bool {
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
    let torn = paging::identity_pa(ident_range_el1_path as *const () as usize as u64) & !0xfff;
    exception::arm_ident_el0();
    unsafe {
        exception::eret_to_el0(black_box(va), torn, user_sp);
    }
    let ok = exception::ident_el0_caught();
    let _ = paging::unmap_page(va);
    frame::free(code_pa);
    ok
}

fn run_probe() -> bool {
    if !paging::mmu_enabled() || !paging::high_split_ready() {
        uart::write_str_raw("ident: miss split-ready\n");
        return false;
    }
    if !paging::identity_tear_ready() {
        uart::write_str_raw("ident: miss tear-ready\n");
        return false;
    }
    if !paging::identity_range_ready() {
        uart::write_str_raw("ident: miss range-ready\n");
        return false;
    }
    // Do not require pc_is_high() here. rustc may `BLR` this probe at
    // its identity address (live `.text` stays mapped). The jump is
    // proven by serial `ident: jump` from `kernel_main_high`.
    let va = paging::ident_tear_page();
    if paging::is_mapped(va) || paging::user_mapped(va) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if !paging::high_mapped(paging::to_high_va(va)) {
        uart::write_str_raw("ident: miss high\n");
        return false;
    }
    let second = paging::ident_tear_page() + 4096;
    if paging::is_mapped(second) || paging::user_mapped(second) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if paging::is_mapped(paging::boot_stub_end()) != true {
        uart::write_str_raw("ident: miss stub\n");
        return false;
    }
    if paging::torn_text_pages() < 4 {
        uart::write_str_raw("ident: miss pages\n");
        return false;
    }
    if paging::is_mapped(paging::KERNEL_TEXT) != true {
        uart::write_str_raw("ident: miss stub0\n");
        return false;
    }
    uart::write_str_raw("ident: split\n");
    if !run_ident_fault() {
        uart::write_str_raw("ident: miss fault\n");
        return false;
    }
    if !run_high_after_tear() {
        uart::write_str_raw("ident: miss high-run\n");
        return false;
    }
    if !run_text_fault() {
        uart::write_str_raw("ident: miss text-fault\n");
        return false;
    }
    if !run_text_high() {
        uart::write_str_raw("ident: miss text-high\n");
        return false;
    }
    if !el0_load_torn() {
        uart::write_str_raw("ident: miss el0\n");
        return false;
    }
    true
}

/// Serial proof: split tables + torn identity `.text` + high fetch.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !run_probe() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "ident: ok");
    true
}

#[cfg(test)]
#[test_case]
fn identity_tear_el1_faults_high_stays() {
    assert!(paging::high_split_ready());
    assert!(paging::identity_tear_ready());
    assert!(paging::identity_range_ready());
    assert!(
        run_probe(),
        "EL1 identity fetch of torn .text must fault; high twin + EL0 DABORT"
    );
}
