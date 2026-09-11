//! Identity map + EL1 MMU + 4 KiB map/unmap window (FR-09 / M7),
//! heap NX (ADR-012), linker-stack guards (ADR-014), and the RO+NX
//! text/data split (ADR-015).
//!
//! After `exception::init` the CPU is at EL1. This module installs a
//! 39-bit / 4 KiB / three-level TTBR0 table: a 1 GiB L1 Device-nGnRnE + XN
//! block for MMIO, an L2 (and one straddling L3) identity of virt RAM so
//! `.text`/`.rodata` are RO+X, `.data`/`.bss`/live linker stacks are RW+NX,
//! and `__kernel_end`…RAM-end is PXN, plus one L2/L3 window at `MAP_WINDOW`.
//! See ADR-008, ADR-012, ADR-014, ADR-015.
//!
//! Heap `GlobalAlloc` is `src/heap.rs` (M8). Not a DTB walker. Not Raspberry Pi.
//! ADR-014 punches 4 KiB unmapped holes under the linker stacks.
//! A second L1 (`L1_USER`) is the EL0 TTBR0 window (ADR-013 mile): it maps
//! kernel text/rodata + the exception stack so the lower-EL handler can
//! restore kernel TTBR0, and omits `.data`/`.bss`/heap. Not isolation.

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut};
use spin::Mutex;

use crate::frame;
use crate::uart;

/// Dedicated VA window for 4 KiB map/unmap. Third GiB — not identity RAM.
pub const MAP_WINDOW: u64 = 0x8000_0000;
/// EL0 trampoline page in the map window (UXN-clear, PXN).
pub const EL0_PAGE: u64 = MAP_WINDOW + 2 * 4096;
/// Linker / `-kernel` TEXT_OFFSET on virt (`0x4008_0000`).
pub const KERNEL_TEXT: u64 = 0x4008_0000;
/// ASID programmed into user TTBR0. Not a claim that ASID isolation works.
pub const USER_ASID: u64 = 1;
const WINDOW_PAGES: u64 = 512;
const PAGE: u64 = 4096;
const L2_BLOCK: u64 = 1 << 21;

const DESC_VALID: u64 = 1 << 0;
const DESC_TABLE: u64 = 1 << 1;
const DESC_AF: u64 = 1 << 10;
const DESC_SH_INNER: u64 = 0b11 << 8;
const DESC_SH_OUTER: u64 = 0b10 << 8;
const DESC_UXN: u64 = 1 << 54;
const DESC_PXN: u64 = 1 << 53;
/// AP[2:1] = 0b10: EL1 read-only, EL0 no data access.
const DESC_AP_RO: u64 = 1 << 7;

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
/// Writable pages are treated as XN. Safe only after text is RO (ADR-015).
const SCTLR_WXN: u64 = 1 << 19;

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
/// Extra L3 tables when punching stack guards in an L2 block.
static mut L3_SPLIT0: Table = Table { entries: [0; 512] };
static mut L3_SPLIT1: Table = Table { entries: [0; 512] };
static mut SPLITS_USED: usize = 0;
/// User TTBR0 L1 (ADR-013 mile). Omits kernel `.data` / `.bss` / heap.
static mut L1_USER: Table = Table { entries: [0; 512] };
static mut L2_USER: Table = Table { entries: [0; 512] };
static mut L3_USER: Table = Table { entries: [0; 512] };
static USER_MAP_OK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static TABLES: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    static __data_start: u8;
}

/// First byte of `.data` (page-aligned). `.text`/`.rodata` end here.
pub fn data_start() -> u64 {
    core::ptr::addr_of!(__data_start) as usize as u64
}

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

fn l1_user_pa() -> u64 {
    addr_of!(L1_USER) as usize as u64
}

fn l2_user_pa() -> u64 {
    addr_of!(L2_USER) as usize as u64
}

fn l3_user_pa() -> u64 {
    addr_of!(L3_USER) as usize as u64
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

unsafe fn l1_user_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1_USER.entries).cast::<u64>().add(i)
}

unsafe fn l2_user_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2_USER.entries).cast::<u64>().add(i)
}

unsafe fn l3_user_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3_USER.entries).cast::<u64>().add(i)
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

/// 2 MiB L2 Normal block. `exec` clears PXN so EL1 can fetch (kernel text).
/// `writable` clears AP[2] so EL1 can store (data / stacks / heap).
fn l2_block(pa: u64, exec: bool, writable: bool) -> u64 {
    let mut d = (pa & !(L2_BLOCK - 1))
        | DESC_VALID
        | (ATTR_NORMAL << 2)
        | DESC_AF
        | DESC_SH_INNER
        | DESC_UXN;
    if !exec {
        d |= DESC_PXN;
    }
    if !writable {
        d |= DESC_AP_RO;
    }
    d
}

fn l3_page_flags(pa: u64, exec: bool, writable: bool) -> u64 {
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
    if !writable {
        d |= DESC_AP_RO;
    }
    d
}

/// Map-window pages are data-only (PXN). Not the heap — see `l3_page_flags`.
fn l3_page(pa: u64) -> u64 {
    l3_page_flags(pa, false, true)
}

#[allow(dead_code)]
fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

unsafe fn desc_at(table_pa: u64, index: usize) -> u64 {
    core::ptr::read((table_pa as *const u64).add(index))
}

fn walk_leaf(l1_table: u64, va: u64) -> Option<u64> {
    let l1i = ((va >> 30) & 0x1ff) as usize;
    let l1e = unsafe { desc_at(l1_table, l1i) };
    if l1e & DESC_VALID == 0 {
        return None;
    }
    if l1e & DESC_TABLE == 0 {
        return Some(l1e);
    }
    let l2e = unsafe { desc_at(l1e & !0xfff, ((va >> 21) & 0x1ff) as usize) };
    if l2e & DESC_VALID == 0 {
        return None;
    }
    if l2e & DESC_TABLE == 0 {
        return Some(l2e);
    }
    let l3e = unsafe { desc_at(l2e & !0xfff, ((va >> 12) & 0x1ff) as usize) };
    if l3e & DESC_VALID == 0 {
        return None;
    }
    Some(l3e)
}

/// Walk kernel TTBR0. `Some(true)` = PXN set (EL1 must not execute).
#[allow(dead_code)] // hello W^X probe + `#[test_case]`.
pub fn pxn_for(va: u64) -> Option<bool> {
    let _g = TABLES.lock();
    walk_leaf(l1_pa(), va).map(|d| d & DESC_PXN != 0)
}

/// Walk kernel TTBR0. `Some(true)` = AP[2] set (EL1 read-only).
#[allow(dead_code)]
pub fn readonly_for(va: u64) -> Option<bool> {
    let _g = TABLES.lock();
    walk_leaf(l1_pa(), va).map(|d| d & DESC_AP_RO != 0)
}

#[allow(dead_code)]
pub fn is_executable(va: u64) -> bool {
    matches!(pxn_for(va), Some(false))
}

#[allow(dead_code)]
pub fn is_readonly(va: u64) -> bool {
    matches!(readonly_for(va), Some(true))
}

/// `false` if the walk hits an invalid descriptor (guard holes, unused RAM).
#[allow(dead_code)]
pub fn is_mapped(va: u64) -> bool {
    pxn_for(va).is_some()
}

/// Walk the user TTBR0 tables. Kernel `.data` must be absent.
#[allow(dead_code)]
pub fn user_mapped(va: u64) -> bool {
    let _g = TABLES.lock();
    walk_leaf(l1_user_pa(), va).is_some()
}

#[allow(dead_code)]
pub fn user_map_ready() -> bool {
    USER_MAP_OK.load(core::sync::atomic::Ordering::SeqCst)
}

/// TTBR0 value installed for EL0 (L1_USER PA + ASID). Identity VA==PA.
#[allow(dead_code)]
pub fn user_ttbr0() -> u64 {
    l1_user_pa() | (USER_ASID << 48)
}

#[allow(dead_code)]
pub fn wxn_enabled() -> bool {
    sctlr_el1() & SCTLR_WXN != 0
}

/// ID_AA64MMFR1_EL1.PAN != 0. cortex-a57 is typically 0 (ARMv8.0).
#[allow(dead_code)]
pub fn pan_implemented() -> bool {
    let id: u64;
    unsafe {
        core::arch::asm!("mrs {v}, ID_AA64MMFR1_EL1", v = out(reg) id);
    }
    (id >> 20) & 0xf != 0
}

/// EL0-executable, EL1-NX page (UXN clear, PXN set). Used for the first mile.
fn l3_page_el0_exec(pa: u64) -> u64 {
    (pa & !0xfff)
        | DESC_VALID
        | DESC_TABLE
        | (ATTR_NORMAL << 2)
        | DESC_SH_INNER
        | DESC_AF
        | DESC_PXN
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
    let writable = l2e & DESC_AP_RO == 0;
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
        l3_split_slot(n, p).write(l3_page_flags(page_va, exec, writable));
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

fn page_exec(va: u64, data: u64) -> bool {
    va >= KERNEL_TEXT && va < data
}

fn page_writable(va: u64, data: u64) -> bool {
    va < KERNEL_TEXT || va >= data
}

/// Fill L2 (and one L3) so `.text`/`.rodata` are RO+X and everything else
/// in the 128 MiB guest is RW+NX. Only that 128 MiB is mapped.
unsafe fn fill_ram_wx() {
    let data = data_start();
    let ram_lo = frame::VIRT_RAM_BASE;
    let ram_hi = ram_lo + frame::VIRT_RAM_SIZE;
    let mut va = ram_lo;
    while va < ram_hi {
        let next = va + L2_BLOCK;
        let i = ((va >> 21) & 0x1ff) as usize;
        if next <= KERNEL_TEXT || va >= data {
            // Entire block is data-side (DTB hole, .data/.bss/stacks, or heap).
            l2_ram_slot(i).write(l2_block(va, false, true));
        } else if va >= KERNEL_TEXT && next <= data {
            l2_ram_slot(i).write(l2_block(va, true, false));
        } else {
            for p in 0..512 {
                let page_va = va + p as u64 * PAGE;
                l3_ram_slot(p).write(l3_page_flags(
                    page_va,
                    page_exec(page_va, data),
                    page_writable(page_va, data),
                ));
            }
            l2_ram_slot(i).write(table_desc(l3_ram_pa()));
        }
        va = next;
    }
}

/// User TTBR0: text/rodata + exception stack only, first RAM 2 MiB L3.
/// Shared map-window L2 so `EL0_PAGE` is visible. Omits `.data`/heap.
unsafe fn fill_user_map() -> bool {
    addr_of_mut!(L1_USER.entries).write([0; 512]);
    addr_of_mut!(L2_USER.entries).write([0; 512]);
    addr_of_mut!(L3_USER.entries).write([0; 512]);
    let data = data_start();
    let exc_lo = crate::exception::exc_stack_bottom();
    let exc_hi = crate::exception::exc_stack_top();
    let block_va = frame::VIRT_RAM_BASE;
    if data <= KERNEL_TEXT || data - KERNEL_TEXT > L2_BLOCK {
        return false;
    }
    if exc_lo < block_va || exc_hi > block_va + L2_BLOCK {
        return false;
    }
    for p in 0..512 {
        let page_va = block_va + p as u64 * PAGE;
        if page_va >= KERNEL_TEXT && page_va < data {
            l3_user_slot(p).write(l3_page_flags(page_va, true, false));
        } else if page_va >= exc_lo && page_va < exc_hi {
            l3_user_slot(p).write(l3_page_flags(page_va, false, true));
        } else {
            l3_user_slot(p).write(0);
        }
    }
    let l2i = ((block_va >> 21) & 0x1ff) as usize;
    l2_user_slot(l2i).write(table_desc(l3_user_pa()));
    l1_user_slot(1).write(table_desc(l2_user_pa()));
    // Same map-window L2/L3 as the kernel (EL0 trampoline lives there).
    l1_user_slot(2).write(table_desc(l2_pa()));
    true
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
        // 0x4000_0000–0x7FFF_FFFF: L2 RAM (text RO+X, data/stacks/heap RW+NX).
        l1_slot(1).write(table_desc(l2_ram_pa()));
        fill_ram_wx();
        if !install_stack_guards() {
            // Leave tables as-is; the hello / test probe will fail closed.
        }
        // 0x8000_0000–0xBFFF_FFFF: L2/L3 window for 4 KiB maps.
        l1_slot(2).write(table_desc(l2_pa()));
        l2_slot(0).write(table_desc(l3_pa()));
        let user_ok = fill_user_map();
        USER_MAP_OK.store(user_ok, core::sync::atomic::Ordering::SeqCst);
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
            "msr tpidr_el1, {ttbr}",
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
        // WXN: writable → XN. Text is RO so it stays executable (ADR-015).
        sctlr |= SCTLR_M | SCTLR_C | SCTLR_SA | SCTLR_I | SCTLR_WXN;
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
    assert!(is_readonly(KERNEL_TEXT), "kernel text must be AP RO (ADR-015)");
    assert!(wxn_enabled(), "SCTLR.WXN must be on once text is RO");
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

#[cfg(test)]
#[test_case]
fn data_and_linker_stacks_are_nx_writable() {
    let data = data_start();
    assert!(data > KERNEL_TEXT);
    assert_eq!(data & 0xfff, 0, "__data_start must be 4 KiB aligned");
    assert_eq!(pxn_for(data), Some(true), ".data must be PXN");
    assert_eq!(readonly_for(data), Some(false), ".data must stay writable");
    let stack = crate::exception::thread_stack_bottom();
    assert_eq!(pxn_for(stack), Some(true), "live linker stack must be PXN");
    assert_eq!(readonly_for(stack), Some(false));
}

#[cfg(test)]
#[test_case]
fn user_ttbr0_omits_kernel_data() {
    assert!(user_map_ready(), "user TTBR0 window failed to build");
    assert_eq!(user_ttbr0() >> 48, USER_ASID);
    assert!(user_mapped(KERNEL_TEXT), "user map must include kernel text");
    assert!(
        !user_mapped(data_start()),
        "user TTBR0 must omit kernel .data"
    );
    assert!(!user_mapped(crate::heap::heap_base()));
}
