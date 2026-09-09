//! Identity map + EL1 MMU + 4 KiB map/unmap window (FR-09 / M7).
//!
//! After `exception::init` the CPU is at EL1. This module installs a
//! 39-bit / 4 KiB / three-level TTBR0 table: 1 GiB L1 blocks for MMIO
//! (Device-nGnRnE) and virt RAM (Normal WB), plus one L2/L3 window at
//! `MAP_WINDOW` for a single-page map/unmap probe. See ADR-008.
//!
//! Not `GlobalAlloc` (M8). Not a DTB walker. Not Raspberry Pi.

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut};
use spin::Mutex;

use crate::frame;
use crate::uart;

/// Dedicated VA window for 4 KiB map/unmap. Third GiB — not identity RAM.
pub const MAP_WINDOW: u64 = 0x8000_0000;
const WINDOW_PAGES: u64 = 512;
const PAGE: u64 = 4096;

const DESC_VALID: u64 = 1 << 0;
const DESC_TABLE: u64 = 1 << 1;
const DESC_AF: u64 = 1 << 10;
const DESC_SH_INNER: u64 = 0b11 << 8;
const DESC_SH_OUTER: u64 = 0b10 << 8;
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

unsafe fn l1_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1.entries).cast::<u64>().add(i)
}

unsafe fn l2_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2.entries).cast::<u64>().add(i)
}

unsafe fn l3_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3.entries).cast::<u64>().add(i)
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

fn l3_page(pa: u64) -> u64 {
    (pa & !0xfff)
        | DESC_VALID
        | DESC_TABLE
        | (ATTR_NORMAL << 2)
        | DESC_SH_INNER
        | DESC_AF
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
        // 0x0000_0000–0x3FFF_FFFF: virt MMIO (UART, GIC, flash).
        l1_slot(0).write(l1_block(0x0000_0000, ATTR_DEVICE, false));
        // 0x4000_0000–0x7FFF_FFFF: virt RAM (kernel + frame pool).
        l1_slot(1).write(l1_block(0x4000_0000, ATTR_NORMAL, true));
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
    assert!(unmap_page(va));
    assert_eq!(l3_entry(va), Some(0));
    frame::free(pa);
}
