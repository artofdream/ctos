//! Identity-teardown cuts (ADR-018 + ADR-019 + ADR-020 + ADR-025 + ADR-037 + ADR-038 + ADR-042 wrap + ADR-049).
//!
//! After MMU + high VBAR, the post-MMU continuation jumps to its
//! TTBR1 alias (`ident: jump`), rewrites rustc vtable / fn-pointer
//! words to high aliases (`ident: reloc`), unmaps a dedicated 16 KiB
//! identity text range (`__ident_tear_*`), unmaps live identity
//! `.text` after the `_start` page (`ident: live`, ADR-020), rewrites
//! identity pointers into `.data`/stacks (`ident: data-reloc`),
//! rewrites identity pointers to `.rodata` (`ident: ro-reloc`), unmaps
//! identity `.rodata` (`ident: rodata`, ADR-025), relocates SP to the
//! high twin, then unmaps identity `.data`/`.bss`/linker stacks
//! (`ident: data`, ADR-037). High twins stay (`L2_HIGH_RAM` clone).
//! Identity heap is unmapped after high `GlobalAlloc` VAs
//! (`ident: heap`, ADR-038). Leftover identity frames after the heap
//! are unmapped (`ident: ram`, ADR-049); high twins stay.
//!
//! Probes: EL1 fetch of a torn identity VA faults (`ident: fault`);
//! EL1 fetch of the high twin still runs (`ident: high` / `ident: text`);
//! EL0 load of a torn VA faults (`ident: no el0`); EL1 load of torn
//! `.rodata` faults (`ident: rodata-fault`); high `.rodata` still
//! loads (`ident: rodata-high`); EL1 load of torn `.data` faults
//! (`ident: data-fault`); high `.data` still loads (`ident: data-high`).
//! EL1 load of torn heap faults (`ident: heap-fault`); high heap still
//! loads (`ident: heap-high`). EL1 load of torn leftover RAM faults
//! (`ident: ram-fault`); high leftover RAM still loads
//! (`ident: ram-high`). `_start` / QEMU `-kernel` stay at
//! `0x4008_0000` (`ident: start-stay`, ADR-042 / ADR-047). Not “the kernel
//! moved.” Not “EL0 isolated.” PAN enable stays Planned (ADR-026).

use core::fmt::Write;
use core::hint::black_box;
use core::mem::transmute;

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

/// AArch64 `LDR X1, [X0]`.
const LDR_X1_X0_A64: u32 = 0xF9400001;

/// Known `.rodata` word. Identity load must fault after ADR-025; high load stays.
#[used]
static IDENT_RODATA_MAGIC: u64 = 0x4354_4F53_524F_4441;

/// Known `.data` word. Identity load must fault after ADR-037; high load stays.
#[used]
#[allow(dead_code)]
static mut IDENT_DATA_MAGIC: u64 = 0x4354_4F53_4441_5441;

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

fn rodata_magic_ident() -> u64 {
    paging::identity_pa(core::ptr::addr_of!(IDENT_RODATA_MAGIC) as usize as u64)
}

fn data_magic_ident() -> u64 {
    paging::identity_pa(core::ptr::addr_of!(IDENT_DATA_MAGIC) as usize as u64)
}

fn run_rodata_high() -> bool {
    let ident = rodata_magic_ident();
    if ident < paging::rodata_start() || ident >= paging::ident_tear_page() {
        return false;
    }
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return false;
    }
    let word = unsafe { core::ptr::read_volatile(high as *const u64) };
    if word != IDENT_RODATA_MAGIC {
        return false;
    }
    uart::write_str_raw("ident: rodata-high\n");
    true
}

fn run_rodata_fault() -> bool {
    let ident = rodata_magic_ident();
    if paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_rodata();
    let _ = black_box(unsafe { core::ptr::read_volatile(ident as *const u64) });
    exception::ident_rodata_caught()
}

fn run_data_high() -> bool {
    let ident = data_magic_ident();
    if ident < paging::data_start() || ident >= frame::kernel_end() {
        return false;
    }
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return false;
    }
    let word = unsafe { core::ptr::read_volatile(high as *const u64) };
    if word != 0x4354_4F53_4441_5441 {
        return false;
    }
    uart::write_str_raw("ident: data-high\n");
    true
}

fn run_data_fault() -> bool {
    let ident = data_magic_ident();
    if paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_data();
    let _ = black_box(unsafe { core::ptr::read_volatile(ident as *const u64) });
    exception::ident_data_caught()
}

fn run_heap_high() -> bool {
    let ident = crate::heap::ident_magic_pa();
    if ident == 0 {
        return false;
    }
    let lo = crate::heap::heap_pa();
    let hi = crate::heap::heap_pa_end();
    if ident < lo || ident >= hi {
        return false;
    }
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return false;
    }
    let word = unsafe { core::ptr::read_volatile(high as *const u64) };
    if word != crate::heap::IDENT_MAGIC {
        return false;
    }
    uart::write_str_raw("ident: heap-high\n");
    true
}

fn run_heap_fault() -> bool {
    let ident = crate::heap::heap_pa();
    if ident == 0 || paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_heap();
    let _ = black_box(unsafe { core::ptr::read_volatile(ident as *const u64) });
    exception::ident_heap_caught()
}

fn run_ram_high() -> bool {
    let lo = crate::heap::heap_pa_end();
    let hi = frame::pool_end();
    if lo == 0 || hi <= lo {
        return false;
    }
    // Magic lives on the last leftover page (first page is the bump target).
    let ident = hi - 4096;
    if ident < lo {
        return false;
    }
    let high = paging::to_high_va(ident);
    if !paging::is_high_va(high) || !paging::high_mapped(high) {
        return false;
    }
    if !paging::high_mapped(paging::to_high_va(lo)) {
        return false;
    }
    let word = unsafe { core::ptr::read_volatile(high as *const u64) };
    if word != 0x4354_4F53_5241_4D21 {
        return false;
    }
    uart::write_str_raw("ident: ram-high\n");
    true
}

fn run_ram_fault() -> bool {
    let ident = crate::heap::heap_pa_end();
    if ident == 0 || paging::is_mapped(ident) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_ram();
    let _ = black_box(unsafe { core::ptr::read_volatile(ident as *const u64) });
    exception::ident_ram_caught()
}

/// ADR-087: EL1 `LDR` of a torn leftover identity VA must take a
/// current-EL translation DABORT. Prints `ident: <name>-fault`.
fn run_left_fault(va: u64, name: &str) -> bool {
    if paging::is_mapped(va) || paging::user_mapped(va) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    exception::arm_ident_left();
    let _ = black_box(unsafe { core::ptr::read_volatile(va as *const u64) });
    if !exception::ident_left_caught() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "ident: {}-fault va={:#x}", name, va);
    true
}

/// ADR-087 fail-closed ratchet: every identity (VA == PA) leaf in kernel,
/// user and ASID-B TTBR0 must sit inside `paging::INV_ALLOW` (I1 MMIO
/// block, I3 boot-stub page). A planted identity page must be caught.
fn run_inventory() -> bool {
    #[cfg(feature = "inv-leak-probe")]
    if paging::plant_persistent_leak() {
        uart::write_str_raw("ident: inv-leak-probe planted va=0x40000000\n");
    }
    let inv = paging::identity_inventory(true);
    {
        let mut w = uart::raw();
        let _ = writeln!(
            w,
            "ident: inv k={} u={} a={} leaks={}",
            inv.ranges[0], inv.ranges[1], inv.ranges[2], inv.leaks
        );
    }
    if inv.leaks != 0 || inv.ranges[0] == 0 {
        return false;
    }
    let (k, u, clean) = paging::inventory_negative_probe();
    let plant = frame::VIRT_RAM_BASE;
    let mut w = uart::raw();
    if !k || !u || !clean {
        let _ = writeln!(w, "ident: inv-neg missed k={} u={} clean={}", k, u, clean);
        return false;
    }
    let _ = writeln!(w, "ident: inv-neg k caught va={:#x}", plant);
    let _ = writeln!(w, "ident: inv-neg u caught va={:#x}", plant);
    let _ = writeln!(w, "ident: inv-neg clean");
    let _ = writeln!(w, "ident: inv-ok allow=stub");
    true
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
    // Non-standing short probe: keep IRQ masked (ADR-041).
    unsafe {
        exception::eret_to_el0_masked(black_box(va), torn, user_sp);
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
    if !paging::identity_reloc_ready() {
        uart::write_str_raw("ident: miss reloc-ready\n");
        return false;
    }
    if !paging::identity_live_ready() {
        uart::write_str_raw("ident: miss live-ready\n");
        return false;
    }
    if !paging::identity_rodata_ready() {
        uart::write_str_raw("ident: miss rodata-ready\n");
        return false;
    }
    if !paging::identity_data_ready() {
        uart::write_str_raw("ident: miss data-ready\n");
        return false;
    }
    if !paging::identity_heap_ready() {
        uart::write_str_raw("ident: miss heap-ready\n");
        return false;
    }
    // After ADR-020 the probe itself is only reachable via the high
    // alias. The jump is also proven by serial `ident: jump`.
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
    if paging::is_mapped(paging::boot_stub_end()) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if paging::torn_live_pages() < 8 {
        uart::write_str_raw("ident: miss live-pages\n");
        return false;
    }
    if paging::is_mapped(paging::rodata_start()) || paging::user_mapped(paging::rodata_start())
    {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if !paging::high_mapped(paging::to_high_va(paging::rodata_start())) {
        uart::write_str_raw("ident: miss rodata-high-map\n");
        return false;
    }
    if paging::torn_rodata_pages() < 1 {
        uart::write_str_raw("ident: miss rodata-pages\n");
        return false;
    }
    if paging::is_mapped(paging::data_start()) {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if !paging::high_mapped(paging::to_high_va(paging::data_start())) {
        uart::write_str_raw("ident: miss data-high-map\n");
        return false;
    }
    if paging::torn_data_pages() < 1 {
        uart::write_str_raw("ident: miss data-pages\n");
        return false;
    }
    if !paging::identity_heap_ready() {
        uart::write_str_raw("ident: miss heap-ready\n");
        return false;
    }
    let heap_lo = crate::heap::heap_pa();
    let heap_hi = crate::heap::heap_pa_end();
    if heap_lo == 0
        || heap_hi <= heap_lo
        || !paging::is_high_va(crate::heap::heap_base())
        || paging::is_mapped(heap_lo)
        || paging::user_mapped(heap_lo)
    {
        uart::write_str_raw("ident: leaked\n");
        return false;
    }
    if !paging::high_mapped(paging::to_high_va(heap_lo)) {
        uart::write_str_raw("ident: miss heap-high-map\n");
        return false;
    }
    if paging::torn_heap_pages() < 1 {
        uart::write_str_raw("ident: miss heap-pages\n");
        return false;
    }
    if paging::reloc_count() == 0 {
        uart::write_str_raw("ident: miss reloc-count\n");
        return false;
    }
    if paging::torn_text_pages() < 4 {
        uart::write_str_raw("ident: miss pages\n");
        return false;
    }
    let stub = paging::identity_pa(paging::KERNEL_TEXT);
    if !paging::is_mapped(stub) || !paging::is_executable(stub) {
        uart::write_str_raw("ident: miss stub0\n");
        return false;
    }
    {
        let mut w = uart::raw();
        let _ = writeln!(
            w,
            "ident: start-stay lo={:#x} hi={:#x}",
            stub,
            paging::boot_stub_end()
        );
    }
    if !paging::identity_ram_ready() {
        uart::write_str_raw("ident: miss ram-ready\n");
        return false;
    }
    let ram_lo = crate::heap::heap_pa_end();
    let ram_hi = frame::pool_end();
    if ram_lo == 0
        || ram_hi <= ram_lo
        || paging::is_mapped(ram_lo)
        || paging::user_mapped(ram_lo)
        || paging::torn_ram_pages() < 1
        || !paging::high_mapped(paging::to_high_va(ram_lo))
    {
        uart::write_str_raw("ident: leaked\n");
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
    if !run_rodata_fault() {
        uart::write_str_raw("ident: miss rodata-fault\n");
        return false;
    }
    if !run_rodata_high() {
        uart::write_str_raw("ident: miss rodata-high\n");
        return false;
    }
    if !run_data_fault() {
        uart::write_str_raw("ident: miss data-fault\n");
        return false;
    }
    if !run_data_high() {
        uart::write_str_raw("ident: miss data-high\n");
        return false;
    }
    if !run_heap_fault() {
        uart::write_str_raw("ident: miss heap-fault\n");
        return false;
    }
    if !run_heap_high() {
        uart::write_str_raw("ident: miss heap-high\n");
        return false;
    }
    if !run_ram_fault() {
        uart::write_str_raw("ident: miss ram-fault\n");
        return false;
    }
    if !run_ram_high() {
        uart::write_str_raw("ident: miss ram-high\n");
        return false;
    }
    if !paging::identity_left_ready() {
        uart::write_str_raw("ident: miss left-ready\n");
        return false;
    }
    if !run_left_fault(frame::VIRT_RAM_BASE, "low") {
        uart::write_str_raw("ident: miss low-fault\n");
        return false;
    }
    if !run_left_fault(paging::data_start() - 4096, "tail") {
        uart::write_str_raw("ident: miss tail-fault\n");
        return false;
    }
    if paging::torn_left_pages().2 > 0 && !run_left_fault(paging::kend_tear_lo(), "kend") {
        uart::write_str_raw("ident: miss kend-fault\n");
        return false;
    }
    if !paging::identity_mmio_ready() || !paging::mmio_high_ready() {
        uart::write_str_raw("ident: miss mmio-ready\n");
        return false;
    }
    // PL011 UARTFR (+0x18) through the torn identity VA: translation fault
    // before any device access. The high alias is what printed this line.
    if !run_left_fault(0x0900_0018, "mmio") {
        uart::write_str_raw("ident: miss mmio-fault\n");
        return false;
    }
    if !run_inventory() {
        uart::write_str_raw("ident: inv missed\n");
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
    assert!(paging::identity_reloc_ready());
    assert!(paging::identity_live_ready());
    assert!(paging::identity_rodata_ready());
    assert!(paging::identity_data_ready());
    assert!(paging::identity_heap_ready());
    assert!(paging::identity_ram_ready());
    assert!(
        run_probe(),
        "EL1 identity fetch of torn .text must fault; high twin + EL0 DABORT; torn .rodata/.data/.heap/.ram"
    );
}

#[cfg(test)]
#[test_case]
fn identity_heap_torn_high_stays() {
    assert!(paging::identity_heap_ready());
    assert!(
        paging::is_high_va(crate::heap::heap_base()),
        "GlobalAlloc must return TTBR1 VAs"
    );
    assert!(
        !paging::is_mapped(crate::heap::heap_pa()),
        "identity heap must be unmapped"
    );
    assert!(paging::high_mapped(paging::to_high_va(crate::heap::heap_pa())));
    assert!(paging::torn_heap_pages() >= 1);
}

#[cfg(test)]
#[test_case]
fn identity_data_torn_high_stays() {
    assert!(paging::identity_data_ready());
    assert!(!paging::is_mapped(paging::data_start()));
    assert!(paging::high_mapped(paging::to_high_va(paging::data_start())));
    assert!(paging::torn_data_pages() >= 1);
}

/// ADR-042 / ADR-049 honesty: `_start` page stays while live image + heap + leftover RAM are torn.
#[cfg(test)]
#[test_case]
fn identity_boot_stub_stays_while_live_torn() {
    assert!(paging::identity_live_ready());
    assert!(paging::identity_rodata_ready());
    assert!(paging::identity_data_ready());
    assert!(paging::identity_heap_ready());
    assert!(paging::identity_ram_ready());
    let stub = paging::identity_pa(paging::KERNEL_TEXT);
    assert!(
        paging::is_mapped(stub),
        "0x4008_0000 boot stub must stay identity-mapped"
    );
    assert!(paging::is_executable(stub));
    assert_eq!(stub, paging::boot_stub_end() - 4096);
    assert!(
        !paging::is_mapped(paging::boot_stub_end()),
        "live identity .text after stub must be torn"
    );
    assert!(!paging::is_mapped(paging::rodata_start()));
    assert!(!paging::is_mapped(paging::data_start()));
    assert!(!paging::is_mapped(crate::heap::heap_pa()));
    let ram_lo = crate::heap::heap_pa_end();
    assert!(ram_lo != 0);
    assert!(
        !paging::is_mapped(ram_lo),
        "leftover identity RAM after the heap must be torn (ADR-049)"
    );
    assert!(paging::high_mapped(paging::to_high_va(ram_lo)));
    assert!(paging::torn_ram_pages() >= 1);
}

#[cfg(test)]
#[test_case]
fn identity_ram_torn_high_stays() {
    assert!(paging::identity_ram_ready());
    assert!(!paging::is_mapped(crate::heap::heap_pa_end()));
    assert!(paging::high_mapped(paging::to_high_va(crate::heap::heap_pa_end())));
    assert!(paging::torn_ram_pages() >= 1);
    let stub = paging::identity_pa(paging::KERNEL_TEXT);
    assert!(paging::is_mapped(stub), "boot stub must stay");
}

/// ADR-087: low RAM, image padding tail and pre-heap frames are torn from
/// kernel and user TTBR0; `_start` stays.
#[cfg(test)]
#[test_case]
fn identity_leftovers_torn_stub_stays() {
    assert!(paging::identity_left_ready());
    for (lo, hi) in paging::leftover_ranges() {
        let mut va = lo;
        while va < hi {
            assert!(!paging::is_mapped(va), "ADR-087 leftover still identity-mapped");
            assert!(!paging::user_mapped(va), "ADR-087 leftover still in user TTBR0");
            va += 4096;
        }
    }
    let (low, tail, _kend) = paging::torn_left_pages();
    assert!(low >= 1 && tail >= 1);
    let stub = paging::identity_pa(paging::KERNEL_TEXT);
    assert!(paging::is_mapped(stub) && paging::is_executable(stub), "boot stub must stay");
}

/// ADR-087: the identity inventory is allowlist-only and catches a plant.
#[cfg(test)]
#[test_case]
fn identity_inventory_allowlist_only_catches_plant() {
    let inv = paging::identity_inventory(false);
    assert_eq!(inv.leaks, 0, "identity leaf outside the allowlist (MMIO block, stub page)");
    assert_eq!(inv.ranges[0], 1, "kernel TTBR0: only the _start stub page (ADR-088)");
    let (k, u, clean) = paging::inventory_negative_probe();
    assert!(k, "kernel TTBR0 plant not caught");
    assert!(u, "user TTBR0 plant not caught");
    assert!(clean, "plant not removed");
}

/// ADR-088: identity MMIO block torn; drivers use the TTBR1 Device alias.
#[cfg(test)]
#[test_case]
fn identity_mmio_torn_high_alias_xn() {
    assert!(paging::mmio_high_ready());
    assert!(paging::identity_mmio_ready());
    for pa in [0x0800_0000u64, 0x0900_0000, 0x0a00_0000] {
        assert!(!paging::is_mapped(pa), "identity MMIO still mapped");
        assert!(paging::high_mapped(paging::to_high_va(pa)), "MMIO high alias missing");
        assert!(paging::mmio_xn(pa));
    }
    assert_eq!(paging::mmio_va(0x0900_0000), paging::to_high_va(0x0900_0000) as usize);
}
