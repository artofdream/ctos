//! Identity map + EL1 MMU + 4 KiB map/unmap window (FR-09 / M7)
//! and W^X split for heap / frame-pool RAM (NFR-10 / ADR-012).
//!
//! After `exception::init` the CPU is at EL1. This module installs a
//! 39-bit / 4 KiB / three-level TTBR0 table: a 1 GiB L1 Device-nGnRnE + XN
//! block for MMIO, an L2 (and one straddling L3) identity of virt RAM so
//! kernel-image pages stay executable while `__kernel_end`…RAM-end is PXN,
//! plus one L2/L3 window at `MAP_WINDOW` for a single-page map/unmap probe.
//! See ADR-008 and ADR-012.
//!
//! Heap `GlobalAlloc` is `src/heap.rs` (M8). Not a DTB walker. Not Raspberry Pi.
//! Linker SP_EL0 / SP_EL1 / fatal stacks stay in the executable image;
//! ADR-014 punches 4 KiB unmapped holes under them.

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut};
use spin::Mutex;

use crate::frame;
use crate::uart;

/// Dedicated VA window for 4 KiB map/unmap. Third GiB — not identity RAM.
pub const MAP_WINDOW: u64 = 0x8000_0000;
/// EL0 trampoline page in the map window (UXN-clear, PXN). Not a user ASID.
pub const EL0_PAGE: u64 = MAP_WINDOW + 2 * 4096;
/// Linker / `-kernel` TEXT_OFFSET on virt (`0x4008_0000`).
pub const KERNEL_TEXT: u64 = 0x4008_0000;
const WINDOW_PAGES: u64 = 512;
const PAGE: u64 = 4096;
const L2_BLOCK: u64 = 1 << 21;

const DESC_VALID: u64 = 1 << 0;
const DESC_TABLE: u64 = 1 << 1;
const DESC_AF: u64 = 1 << 10;
const DESC_SH_INNER: u64 = 0b11 << 8;
const DESC_SH_OUTER: u64 = 0b10 << 8;
/// AP[2:1] = 01: EL1/EL0 read-write. An EL0 instruction fetch is an unprivileged read.
const DESC_AP_EL0: u64 = 0b01 << 6;
const DESC_UXN: u64 = 1 << 54;
const DESC_PXN: u64 = 1 << 53;

const ATTR_DEVICE: u64 = 0;
const ATTR_NORMAL: u64 = 1;

const MAIR: u64 = 0x00 | (0xFF << 8);

const TCR_T0SZ: u64 = 25;
const TCR_IRGN0_WBWA: u64 = 0b01 << 8;
const TCR_ORGN0_WBWA: u64 = 0b01 << 10;
const TCR_SH0_INNER: u64 = 0b11 << 12;
const TCR_EPD1: u64 = 1 << 23;
const TCR_IPS_40: u64 = 0b010 << 32;

const SCTLR_M: u64 = 1 << 0;
const SCTLR_C: u64 = 1 << 2;
const SCTLR_SA: u64 = 1 << 3;
const SCTLR_I: u64 = 1 << 12;

const PROBE_MAGIC: u64 = 0x4354_4F53; // "CTOS"

/// `spin::Mutex` prefixes an atomic, so tables cannot live inside it
/// (TTBR0 / table descriptors must be 4 KiB-aligned). Single-core:
/// take `TABLES` before touching these.
#[repr(C, align(4096))]
struct Table {
    entries: [u64; 512],
}

static mut L1: Table = Table { entries: [0; 512] };
static mut L2: Table = Table { entries: [0; 512] };
static mut L3: Table = Table { entries: [0; 512] };
/// L2 for the RAM GiB (`0x4000_0000`). Replaces the M7 executable L1 block.
static mut L2_RAM: Table = Table { entries: [0; 512] };
/// L3 for the single 2 MiB that straddles `__kernel_end` (X vs PXN).
static mut L3_RAM: Table = Table { entries: [0; 512] };
/// Extra L3 tables when punching stack guards in an executable L2 block.
static mut L3_SPLIT0: Table = Table { entries: [0; 512] };
static mut L3_SPLIT1: Table = Table { entries: [0; 512] };
static mut SPLITS_USED: usize = 0;
static TABLES: Mutex<()> = Mutex::new(());

fn l1_pa() -> u64 {
    addr_of!(L1) as usize as u64
}

fn l2_pa() -> u64 {
    addr_of!(L2) as usize as u64
}

fn l3_pa() -> u64 {
    addr_of!(L3) as usize as u64
}

fn l2_ram_pa() -> u64 {
    addr_of!(L2_RAM) as usize as u64
}

fn l3_ram_pa() -> u64 {
    addr_of!(L3_RAM) as usize as u64
}

fn l3_split0_pa() -> u64 {
    addr_of!(L3_SPLIT0) as usize as u64
}

fn l3_split1_pa() -> u64 {
    addr_of!(L3_SPLIT1) as usize as u64
}

unsafe fn l1_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1.entries).cast::<u64>().add(i)
}

unsafe fn l2_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2.entries).cast::<u64>().add(i)
}

unsafe fn l3_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3.entries).cast::<u64>().add(i)
}

unsafe fn l2_ram_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2_RAM.entries).cast::<u64>().add(i)
}

unsafe fn l3_ram_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3_RAM.entries).cast::<u64>().add(i)
}

unsafe fn l3_split_slot(which: usize, i: usize) -> *mut u64 {
    if which == 0 {
        addr_of_mut!(L3_SPLIT0.entries).cast::<u64>().add(i)
    } else {
        addr_of_mut!(L3_SPLIT1.entries).cast::<u64>().add(i)
    }
}

fn l1_block(pa: u64, attr: u64, exec: bool) -> u64 {
    let mut d = (pa & !((1u64 << 30) - 1)) | DESC_VALID | (attr << 2) | DESC_AF;
    if attr == ATTR_DEVICE {
        d |= DESC_SH_OUTER | DESC_UXN | DESC_PXN;
    } else {
        d |= DESC_SH_INNER;
        if !exec {
            d |= DESC_UXN | DESC_PXN;
        }
    }
    d
}

fn table_desc(pa: u64) -> u64 {
    (pa & !0xfff) | DESC_VALID | DESC_TABLE
}

/// 2 MiB L2 Normal block. `exec` clears PXN so EL1 can fetch (kernel image).
fn l2_block(pa: u64, exec: bool) -> u64 {
    let mut d = (pa & !(L2_BLOCK - 1))
        | DESC_VALID
        | (ATTR_NORMAL << 2)
        | DESC_AF
        | DESC_SH_INNER
        | DESC_UXN;
    if !exec {
        d |= DESC_PXN;
    }
    d
}

fn l3_page_flags(pa: u64, exec: bool) -> u64 {
    let mut d = (pa & !0xfff)
        | DESC_VALID
        | DESC_TABLE
        | (ATTR_NORMAL << 2)
        | DESC_SH_INNER
        | DESC_AF
        | DESC_UXN;
    if !exec {
        d |= DESC_PXN;
    }
    d
}

/// Map-window pages are data-only (PXN). Not the heap — see `l3_page_flags`.
fn l3_page(pa: u64) -> u64 {
    l3_page_flags(pa, false)
}

fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

unsafe fn desc_at(table_pa: u64, index: usize) -> u64 {
    core::ptr::read((table_pa as *const u64).add(index))
}

/// Walk TTBR0. `Some(true)` = PXN set (EL1 must not execute).
#[allow(dead_code)] // hello W^X probe + `#[test_case]`.
pub fn pxn_for(va: u64) -> Option<bool> {
    let _g = TABLES.lock();
    let l1i = ((va >> 30) & 0x1ff) as usize;
    let l1e = unsafe { l1_slot(l1i).read() };
    if l1e & DESC_VALID == 0 {
        return None;
    }
    if l1e & DESC_TABLE == 0 {
        return Some(l1e & DESC_PXN != 0);
    }
    let l2e = unsafe { desc_at(l1e & !0xfff, ((va >> 21) & 0x1ff) as usize) };
    if l2e & DESC_VALID == 0 {
        return None;
    }
    if l2e & DESC_TABLE == 0 {
        return Some(l2e & DESC_PXN != 0);
    }
    let l3e = unsafe { desc_at(l2e & !0xfff, ((va >> 12) & 0x1ff) as usize) };
    if l3e & DESC_VALID == 0 {
        return None;
    }
    Some(l3e & DESC_PXN != 0)
}

#[allow(dead_code)]
pub fn is_executable(va: u64) -> bool {
    matches!(pxn_for(va), Some(false))
}

/// `false` if the walk hits an invalid descriptor (guard holes, unused RAM).
#[allow(dead_code)]
pub fn is_mapped(va: u64) -> bool {
    pxn_for(va).is_some()
}

/// EL0-executable, EL1-NX page (UXN clear, PXN set, AP[2:1]=01). Used for the first mile.
fn l3_page_el0_exec(pa: u64) -> u64 {
    (pa & !0xfff)
        | DESC_VALID
        | DESC_TABLE
        | (ATTR_NORMAL << 2)
        | DESC_SH_INNER
        | DESC_AF
        | DESC_PXN
        | DESC_AP_EL0
}

/// Split an L2 block into L3 so a single 4 KiB slot can be invalidated.
/// Returns the L3 table PA, or `None` if the slot is empty or we are out of tables.
unsafe fn l3_pa_for_ram_block(l2_index: usize) -> Option<u64> {
    let l2e = l2_ram_slot(l2_index).read();
    if l2e & DESC_VALID == 0 {
        return None;
    }
    if l2e & DESC_TABLE != 0 {
        return Some(l2e & !0xfff);
    }
    let exec = l2e & DESC_PXN == 0;
    let block_va = frame::VIRT_RAM_BASE + (l2_index as u64) * L2_BLOCK;
    let n = SPLITS_USED;
    if n >= 2 {
        return None;
    }
    SPLITS_USED = n + 1;
    let l3_pa = if n == 0 {
        l3_split0_pa()
    } else {
        l3_split1_pa()
    };
    for p in 0..512 {
        let page_va = block_va + p as u64 * PAGE;
        l3_split_slot(n, p).write(l3_page_flags(page_va, exec));
    }
    l2_ram_slot(l2_index).write(table_desc(l3_pa));
    Some(l3_pa)
}

/// Clear the identity L3 slot for `va` (4 KiB aligned). Used for stack guards.
unsafe fn unmap_identity_page(va: u64) -> bool {
    if va & (PAGE - 1) != 0 {
        return false;
    }
    let l2i = ((va >> 21) & 0x1ff) as usize;
    let Some(l3_pa) = l3_pa_for_ram_block(l2i) else {
        return false;
    };
    let l3i = ((va >> 12) & 0x1ff) as usize;
    core::ptr::write((l3_pa as *mut u64).add(l3i), 0);
    true
}

/// Punch the three linker-stack guard holes (ADR-014). Call before MMU on.
unsafe fn install_stack_guards() -> bool {
    let mut ok = true;
    for va in crate::exception::stack_guards() {
        if !unmap_identity_page(va) {
            ok = false;
        }
    }
    ok
}

/// Fill L2 (and one L3) so `[KERNEL_TEXT, kernel_end)` is executable and
/// `[kernel_end, RAM end)` is PXN. Only the 128 MiB guest is mapped.
unsafe fn fill_ram_wx() {
    let kend = align_up(frame::kernel_end(), PAGE);
    let ram_lo = frame::VIRT_RAM_BASE;
    let ram_hi = ram_lo + frame::VIRT_RAM_SIZE;
    let mut va = ram_lo;
    while va < ram_hi {
        let next = va + L2_BLOCK;
        let i = ((va >> 21) & 0x1ff) as usize;
        if next <= kend {
            l2_ram_slot(i).write(l2_block(va, true));
        } else if va >= kend {
            l2_ram_slot(i).write(l2_block(va, false));
        } else {
            for p in 0..512 {
                let page_va = va + p as u64 * PAGE;
                let exec = page_va >= KERNEL_TEXT && page_va < kend;
                l3_ram_slot(p).write(l3_page_flags(page_va, exec));
            }
            l2_ram_slot(i).write(table_desc(l3_ram_pa()));
        }
        va = next;
    }
}

fn dsb_ish() {
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }
}

fn isb() {
    unsafe {
        core::arch::asm!("isb", options(nostack, preserves_flags));
    }
}

fn tlbi_all() {
    unsafe {
        core::arch::asm!("tlbi vmalle1", options(nostack, preserves_flags));
    }
    dsb_ish();
    isb();
}

fn tlbi_va(va: u64) {
    let page = va >> 12;
    unsafe {
        core::arch::asm!(
            "tlbi vaae1, {x}",
            x = in(reg) page,
            options(nostack, preserves_flags),
        );
    }
    dsb_ish();
    isb();
}

fn window_index(va: u64) -> Option<usize> {
    if va < MAP_WINDOW {
        return None;
    }
    let off = va - MAP_WINDOW;
    if off % PAGE != 0 || off / PAGE >= WINDOW_PAGES {
        return None;
    }
    Some((off / PAGE) as usize)
}

/// Fill identity L1 blocks + the map window, then turn the MMU on.
pub fn init() {
    let _g = TABLES.lock();
    unsafe {
        addr_of_mut!(L1.entries).write([0; 512]);
        addr_of_mut!(L2.entries).write([0; 512]);
        addr_of_mut!(L3.entries).write([0; 512]);
        addr_of_mut!(L2_RAM.entries).write([0; 512]);
        addr_of_mut!(L3_RAM.entries).write([0; 512]);
        addr_of_mut!(L3_SPLIT0.entries).write([0; 512]);
        addr_of_mut!(L3_SPLIT1.entries).write([0; 512]);
        SPLITS_USED = 0;
        // 0x0000_0000–0x3FFF_FFFF: virt MMIO (UART, GIC, flash).
        l1_slot(0).write(l1_block(0x0000_0000, ATTR_DEVICE, false));
        // 0x4000_0000–0x7FFF_FFFF: L2 RAM (kernel X, heap/frames PXN).
        l1_slot(1).write(table_desc(l2_ram_pa()));
        fill_ram_wx();
        if !install_stack_guards() {
            // Leave tables as-is; the hello / test probe will fail closed.
        }
        // 0x8000_0000–0xBFFF_FFFF: L2/L3 window for 4 KiB maps.
        l1_slot(2).write(table_desc(l2_pa()));
        l2_slot(0).write(table_desc(l3_pa()));
    }
    dsb_ish();

    let tcr = TCR_T0SZ
        | TCR_IRGN0_WBWA
        | TCR_ORGN0_WBWA
        | TCR_SH0_INNER
        | TCR_EPD1
        | TCR_IPS_40;
    let ttbr = l1_pa();
    unsafe {
        core::arch::asm!(
            "msr mair_el1, {mair}",
            "msr tcr_el1, {tcr}",
            "msr ttbr0_el1, {ttbr}",
            "isb",
            mair = in(reg) MAIR,
            tcr = in(reg) tcr,
            ttbr = in(reg) ttbr,
        );
    }
    tlbi_all();

    let mut sctlr: u64;
    unsafe {
        core::arch::asm!("mrs {v}, sctlr_el1", v = out(reg) sctlr);
        sctlr |= SCTLR_M | SCTLR_C | SCTLR_SA | SCTLR_I;
        core::arch::asm!(
            "msr sctlr_el1, {v}",
            "isb",
            v = in(reg) sctlr,
        );
    }
}

pub fn mmu_enabled() -> bool {
    sctlr_el1() & SCTLR_M != 0
}

#[allow(dead_code)] // `#[test_case]` + hello probe.
pub fn sctlr_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, sctlr_el1", v = out(reg) v);
    }
    v
}

#[allow(dead_code)]
pub fn ttbr0_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, ttbr0_el1", v = out(reg) v);
    }
    v
}

#[allow(dead_code)]
pub fn l1_table_addr() -> u64 {
    l1_pa()
}

/// Map a 4 KiB frame at `va` in the dedicated window as EL0-executable (PXN, UXN clear).
pub fn map_el0_exec(va: u64, pa: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    let _g = TABLES.lock();
    unsafe {
        if l3_slot(i).read() & DESC_VALID != 0 {
            return false;
        }
        l3_slot(i).write(l3_page_el0_exec(pa));
    }
    dsb_ish();
    tlbi_va(va);
    true
}

/// Map a 4 KiB frame at `va` in the dedicated window.
pub fn map_page(va: u64, pa: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    let _g = TABLES.lock();
    unsafe {
        if l3_slot(i).read() & DESC_VALID != 0 {
            return false;
        }
        l3_slot(i).write(l3_page(pa));
    }
    dsb_ish();
    tlbi_va(va);
    true
}

/// Clear the L3 slot for `va` and invalidate that translation.
pub fn unmap_page(va: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    let _g = TABLES.lock();
    unsafe {
        if l3_slot(i).read() & DESC_VALID == 0 {
            return false;
        }
        l3_slot(i).write(0);
    }
    dsb_ish();
    tlbi_va(va);
    true
}

#[allow(dead_code)]
pub fn l3_entry(va: u64) -> Option<u64> {
    let i = window_index(va)?;
    let _g = TABLES.lock();
    Some(unsafe { l3_slot(i).read() })
}

/// Serial proof: MMU on, alloc, map, write/read, unmap.
///
/// Does not touch an unmapped VA (that would take an unhandled abort).
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !mmu_enabled() {
        return false;
    }
    let Some(pa) = frame::alloc() else {
        return false;
    };
    let va = MAP_WINDOW;
    if !map_page(va, pa) {
        frame::free(pa);
        return false;
    }
    let via_window = va as *mut u64;
    let via_ident = pa as *mut u64;
    unsafe {
        core::ptr::write_volatile(via_window, PROBE_MAGIC);
        if core::ptr::read_volatile(via_window) != PROBE_MAGIC {
            let _ = unmap_page(va);
            frame::free(pa);
            return false;
        }
        if core::ptr::read_volatile(via_ident) != PROBE_MAGIC {
            let _ = unmap_page(va);
            frame::free(pa);
            return false;
        }
    }
    if !unmap_page(va) {
        frame::free(pa);
        return false;
    }
    if l3_entry(va) != Some(0) {
        frame::free(pa);
        return false;
    }
    frame::free(pa);
    let mut w = uart::raw();
    let _ = writeln!(w, "paging: ok");
    true
}

#[cfg(test)]
#[test_case]
fn mmu_is_enabled() {
    assert!(mmu_enabled());
    assert_eq!(ttbr0_el1() & !0xfff, l1_table_addr());
}

#[cfg(test)]
#[test_case]
fn map_unmap_roundtrip() {
    let pa = frame::alloc().expect("frame");
    let va = MAP_WINDOW + PAGE;
    assert!(map_page(va, pa));
    unsafe {
        core::ptr::write_volatile(va as *mut u64, PROBE_MAGIC);
        assert_eq!(core::ptr::read_volatile(va as *const u64), PROBE_MAGIC);
        assert_eq!(core::ptr::read_volatile(pa as *const u64), PROBE_MAGIC);
    }
    assert!(l3_entry(va).unwrap() & DESC_VALID != 0);
    assert!(l3_entry(va).unwrap() & DESC_PXN != 0, "map window must be PXN");
    assert!(unmap_page(va));
    assert_eq!(l3_entry(va), Some(0));
    frame::free(pa);
}

#[cfg(test)]
#[test_case]
fn kernel_text_is_executable() {
    assert!(is_executable(KERNEL_TEXT), "kernel TEXT_OFFSET must be PXN-clear");
    assert!(is_executable(crate::exception::vector_table_addr()));
}

#[cfg(test)]
#[test_case]
fn heap_and_mmio_are_pxn() {
    assert_eq!(pxn_for(crate::heap::heap_base()), Some(true));
    assert_eq!(pxn_for(crate::heap::heap_end() - 1), Some(true));
    assert_eq!(pxn_for(frame::kernel_end()), Some(true));
    // PL011 is in the Device L1 block (already XN from M7).
    assert_eq!(pxn_for(0x0900_0000), Some(true));
}
