//! Identity map + EL1 MMU + 4 KiB map/unmap window (FR-09 / M7),
//! heap NX (ADR-012), linker-stack guards (ADR-014), and the RO+NX
//! text/data split (ADR-015).
//!
//! After `exception::init` the CPU is at EL1. This module installs a
//! 39-bit / 4 KiB / three-level TTBR0 table: a 1 GiB L1 Device-nGnRnE + XN
//! block for MMIO, an L2 (and **one L3 per straddling 2 MiB**) identity of
//! virt RAM so `.text`/`.rodata` are RO+X, `.data`/`.bss`/live linker
//! stacks are RW+NX, and `__kernel_end`…RAM-end is PXN, plus one L2/L3
//! window at `MAP_WINDOW`. See ADR-008, ADR-012, ADR-014, ADR-015.
//!
//! A single shared L3 cannot describe two 2 MiB blocks. When
//! `__data_start` leaves the first RAM 2 MiB, KERNEL_TEXT and `.data`
//! need distinct L3 tables. The user TTBR0 (`L1_USER`) maps every 2 MiB
//! that holds kernel text or the exception stack — not only the first
//! RAM block. ASID isolation (`src/asid.rs`) uses a second L1 with `nG`
//! probe pages and does **not** `TLBI VMALLE1` on the switch.
//!
//! TTBR1 (ADR-016 first cut + ADR-017 exec mile) is enabled with the
//! same 39-bit T1SZ. One kernel-private page lives at `TTBR1_PRIV`
//! (EL1 RW, EL0 none). `L1_HIGH[1]` aliases the identity RAM tables so
//! EL1 can fetch `.text` at `identity + TTBR1_BASE`. `VBAR_EL1` is
//! moved to that high alias after the MMU is on. `_start` and the
//! QEMU `-kernel` load stay at `0x4008_0000`. Full identity teardown
//! is Planned. Not a DTB walker. Not Raspberry Pi. Not “EL0 isolated.”

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
/// Second ASID for the dual-table isolation probe (ADR-013). Not PAN.
pub const ASID_ISO_B: u64 = 2;
/// Map-window VA mapped to different PAs under ASID 1 vs ASID 2 (`nG`).
pub const ASID_DUAL_VA: u64 = MAP_WINDOW + 5 * 4096;
/// Map-window VA mapped only under ASID 1. ASID 2 must fault (stale = fail).
pub const ASID_CONFLICT_VA: u64 = MAP_WINDOW + 6 * 4096;
/// First VA of the TTBR1 / high-half window (T1SZ=25, 39-bit).
pub const TTBR1_BASE: u64 = 0xFFFF_FF80_0000_0000;
/// Add this to a TTBR0 identity VA to get the TTBR1 RAM alias (ADR-017).
pub const TTBR1_OFFSET: u64 = TTBR1_BASE;
/// Kernel-private page in TTBR1 (EL1 RW, EL0 none). Not a relocated kernel.
pub const TTBR1_PRIV: u64 = TTBR1_BASE;
/// 39-bit input address mask (T0SZ = T1SZ = 25).
const VA_IA_MASK: u64 = (1u64 << 39) - 1;
/// nG (not global): TLB entry is ASID-tagged. Global entries ignore ASID.
const DESC_NG: u64 = 1 << 11;
const WINDOW_PAGES: u64 = 512;
const PAGE: u64 = 4096;
const L2_BLOCK: u64 = 1 << 21;
/// Enough L3s for image straddles + guard splits on a multi-block layout.
const L3_KERNEL_CAP: usize = 16;
/// User TTBR0 L3s: text and exception-stack blocks that are not uniform L2.
const L3_USER_CAP: usize = 8;

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
const TCR_T1SZ: u64 = 25 << 16;
const TCR_IRGN1_WBWA: u64 = 0b01 << 24;
const TCR_ORGN1_WBWA: u64 = 0b01 << 26;
const TCR_SH1_INNER: u64 = 0b11 << 28;
/// Set = TTBR1 walks disabled. We leave this clear (ADR-016).
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
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
struct Table {
    entries: [u64; 512],
}

const fn empty_table() -> Table {
    Table { entries: [0; 512] }
}

static mut L1: Table = empty_table();
static mut L2: Table = empty_table();
static mut L3: Table = empty_table();
/// L2 for the RAM GiB (`0x4000_0000`). Replaces the M7 executable L1 block.
static mut L2_RAM: Table = empty_table();
/// One L3 per kernel 2 MiB that is not a uniform L2 block (straddle or guard).
static mut L3_KERNEL: [Table; L3_KERNEL_CAP] = [empty_table(); L3_KERNEL_CAP];
static mut L3_KERNEL_USED: usize = 0;
/// User TTBR0 L1 (ADR-013 mile). Omits kernel `.data` / `.bss` / heap.
static mut L1_USER: Table = empty_table();
static mut L2_USER: Table = empty_table();
static mut L3_USER: [Table; L3_USER_CAP] = [empty_table(); L3_USER_CAP];
static mut L3_USER_USED: usize = 0;
/// ASID-B tables: same MMIO + RAM L2 as the kernel, own map-window L2/L3.
static mut L1_ASID_B: Table = empty_table();
static mut L2_ASID_B: Table = empty_table();
static mut L3_ASID_B: Table = empty_table();
/// TTBR1 L1/L2/L3 for the kernel-private high page (ADR-016 first cut).
/// L1_HIGH[1] aliases `L2_RAM` so kernel text is fetchable at high VA.
static mut L1_HIGH: Table = empty_table();
static mut L2_HIGH: Table = empty_table();
static mut L3_HIGH: Table = empty_table();
static USER_MAP_OK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static TTBR1_OK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static HIGH_ALIAS_OK: core::sync::atomic::AtomicBool =
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

fn l3_kernel_pa(i: usize) -> u64 {
    unsafe { addr_of!(L3_KERNEL[i]) as usize as u64 }
}

fn l1_user_pa() -> u64 {
    addr_of!(L1_USER) as usize as u64
}

fn l2_user_pa() -> u64 {
    addr_of!(L2_USER) as usize as u64
}

fn l3_user_pa(i: usize) -> u64 {
    unsafe { addr_of!(L3_USER[i]) as usize as u64 }
}

fn l1_asid_b_pa() -> u64 {
    addr_of!(L1_ASID_B) as usize as u64
}

fn l2_asid_b_pa() -> u64 {
    addr_of!(L2_ASID_B) as usize as u64
}

fn l3_asid_b_pa() -> u64 {
    addr_of!(L3_ASID_B) as usize as u64
}

fn l1_high_pa() -> u64 {
    addr_of!(L1_HIGH) as usize as u64
}

fn l2_high_pa() -> u64 {
    addr_of!(L2_HIGH) as usize as u64
}

fn l3_high_pa() -> u64 {
    addr_of!(L3_HIGH) as usize as u64
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

unsafe fn l1_user_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1_USER.entries).cast::<u64>().add(i)
}

unsafe fn l2_user_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2_USER.entries).cast::<u64>().add(i)
}

unsafe fn l1_asid_b_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1_ASID_B.entries).cast::<u64>().add(i)
}

unsafe fn l2_asid_b_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2_ASID_B.entries).cast::<u64>().add(i)
}

unsafe fn l3_asid_b_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3_ASID_B.entries).cast::<u64>().add(i)
}

unsafe fn l1_high_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L1_HIGH.entries).cast::<u64>().add(i)
}

unsafe fn l2_high_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L2_HIGH.entries).cast::<u64>().add(i)
}

unsafe fn l3_high_slot(i: usize) -> *mut u64 {
    addr_of_mut!(L3_HIGH.entries).cast::<u64>().add(i)
}

unsafe fn alloc_kernel_l3() -> Option<u64> {
    let n = L3_KERNEL_USED;
    if n >= L3_KERNEL_CAP {
        return None;
    }
    L3_KERNEL_USED = n + 1;
    addr_of_mut!(L3_KERNEL[n].entries).write([0; 512]);
    Some(l3_kernel_pa(n))
}

unsafe fn alloc_user_l3() -> Option<u64> {
    let n = L3_USER_USED;
    if n >= L3_USER_CAP {
        return None;
    }
    L3_USER_USED = n + 1;
    addr_of_mut!(L3_USER[n].entries).write([0; 512]);
    Some(l3_user_pa(n))
}

unsafe fn l3_write(l3_pa: u64, index: usize, desc: u64) {
    core::ptr::write((l3_pa as *mut u64).add(index), desc);
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

/// Identity VA → TTBR1 RAM alias. QEMU `-kernel` / `_start` stay low.
#[allow(dead_code)]
pub fn to_high_va(va: u64) -> u64 {
    (va & VA_IA_MASK).wrapping_add(TTBR1_OFFSET)
}

/// True when `va` is in the TTBR1 window (T1SZ=25).
#[allow(dead_code)]
pub fn is_high_va(va: u64) -> bool {
    va >= TTBR1_BASE
}

/// Walk a 39-bit / 4 KiB table. Masks to the T0SZ/T1SZ input address so
/// a canonical high VA and its identity twin hit the same L1/L2/L3 slots.
fn walk_leaf(l1_table: u64, va: u64) -> Option<u64> {
    let ia = va & VA_IA_MASK;
    let l1i = ((ia >> 30) & 0x1ff) as usize;
    let l1e = unsafe { desc_at(l1_table, l1i) };
    if l1e & DESC_VALID == 0 {
        return None;
    }
    if l1e & DESC_TABLE == 0 {
        return Some(l1e);
    }
    let l2e = unsafe { desc_at(l1e & !0xfff, ((ia >> 21) & 0x1ff) as usize) };
    if l2e & DESC_VALID == 0 {
        return None;
    }
    if l2e & DESC_TABLE == 0 {
        return Some(l2e);
    }
    let l3e = unsafe { desc_at(l2e & !0xfff, ((ia >> 12) & 0x1ff) as usize) };
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

/// Walk TTBR1. The private page is present only after `map_ttbr1_priv`.
/// After ADR-017, RAM identity VAs are also present at `to_high_va(va)`.
#[allow(dead_code)]
pub fn high_mapped(va: u64) -> bool {
    let _g = TABLES.lock();
    walk_leaf(l1_high_pa(), va).is_some()
}

/// Walk TTBR1. `Some(true)` = PXN set on the high alias.
#[allow(dead_code)]
pub fn high_pxn_for(va: u64) -> Option<bool> {
    if !is_high_va(va) {
        return None;
    }
    let _g = TABLES.lock();
    walk_leaf(l1_high_pa(), va).map(|d| d & DESC_PXN != 0)
}

#[allow(dead_code)]
pub fn high_is_executable(va: u64) -> bool {
    matches!(high_pxn_for(va), Some(false))
}

/// TTBR1 tables programmed and walks enabled (ADR-016 first cut).
#[allow(dead_code)]
pub fn ttbr1_ready() -> bool {
    TTBR1_OK.load(core::sync::atomic::Ordering::SeqCst)
}

/// Identity RAM is aliased at `va + TTBR1_BASE` (ADR-017).
#[allow(dead_code)]
pub fn high_alias_ready() -> bool {
    HIGH_ALIAS_OK.load(core::sync::atomic::Ordering::SeqCst)
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

/// Kernel L1 with ASID 0 (boot TTBR0). Restore here after the ASID probe.
#[allow(dead_code)]
pub fn kernel_ttbr0() -> u64 {
    l1_pa()
}

/// Kernel L1 tagged with the user ASID. Used as ASID-A in the isolation probe.
#[allow(dead_code)]
pub fn asid_a_ttbr0() -> u64 {
    l1_pa() | (USER_ASID << 48)
}

/// Alternate L1 + ASID 2. Shares RAM/MMIO; own map-window leaves.
#[allow(dead_code)]
pub fn asid_b_ttbr0() -> u64 {
    l1_asid_b_pa() | (ASID_ISO_B << 48)
}

/// TTBR1 L1 PA (ASID comes from TTBR0; TCR.A1 stays 0).
#[allow(dead_code)]
pub fn kernel_ttbr1() -> u64 {
    l1_high_pa()
}

/// Switch TTBR0 without `TLBI VMALLE1`. Isolation mile: programming + no full flush.
#[allow(dead_code)]
pub fn switch_ttbr0_no_tlbi(ttbr: u64) {
    dsb_ish();
    unsafe {
        core::arch::asm!(
            "msr ttbr0_el1, {t}",
            "isb",
            t = in(reg) ttbr,
            options(nostack, preserves_flags),
        );
    }
}

/// Invalidate one VA in every ASID (`TLBI VAAE1`). Setup / unmap only.
#[allow(dead_code)]
pub fn invalidate_va(va: u64) {
    tlbi_va(va);
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

/// First byte past the first RAM 2 MiB (`0x4020_0000`). Layout ratchet.
#[allow(dead_code)]
pub fn first_ram_l2_end() -> u64 {
    frame::VIRT_RAM_BASE + L2_BLOCK
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
    let l3_pa = alloc_kernel_l3()?;
    for p in 0..512 {
        let page_va = block_va + p as u64 * PAGE;
        l3_write(l3_pa, p, l3_page_flags(page_va, exec, writable));
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
    l3_write(l3_pa, l3i, 0);
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

/// Fill L2 (and one fresh L3 per mixed 2 MiB) so `.text`/`.rodata` are
/// RO+X and everything else in the 128 MiB guest is RW+NX.
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
        } else if let Some(l3) = alloc_kernel_l3() {
            for p in 0..512 {
                let page_va = va + p as u64 * PAGE;
                l3_write(
                    l3,
                    p,
                    l3_page_flags(
                        page_va,
                        page_exec(page_va, data),
                        page_writable(page_va, data),
                    ),
                );
            }
            l2_ram_slot(i).write(table_desc(l3));
        }
        va = next;
    }
}

fn ranges_overlap(a0: u64, a1: u64, b0: u64, b1: u64) -> bool {
    a0 < b1 && b0 < a1
}

/// User TTBR0: every 2 MiB that holds text/rodata or the exception stack.
/// Shared map-window L2 so `EL0_PAGE` is visible. Omits `.data`/heap.
unsafe fn fill_user_map() -> bool {
    addr_of_mut!(L1_USER.entries).write([0; 512]);
    addr_of_mut!(L2_USER.entries).write([0; 512]);
    L3_USER_USED = 0;
    let data = data_start();
    let exc_lo = crate::exception::exc_stack_bottom();
    let exc_hi = crate::exception::exc_stack_top();
    if data <= KERNEL_TEXT || exc_lo >= exc_hi {
        return false;
    }
    let ram_lo = frame::VIRT_RAM_BASE;
    let ram_hi = ram_lo + frame::VIRT_RAM_SIZE;
    if exc_lo < ram_lo || exc_hi > ram_hi {
        return false;
    }
    let mut va = ram_lo;
    while va < ram_hi {
        let next = va + L2_BLOCK;
        let i = ((va >> 21) & 0x1ff) as usize;
        let overlaps_text = ranges_overlap(va, next, KERNEL_TEXT, data);
        let overlaps_exc = ranges_overlap(va, next, exc_lo, exc_hi);
        if !overlaps_text && !overlaps_exc {
            va = next;
            continue;
        }
        let all_text = va >= KERNEL_TEXT && next <= data && !overlaps_exc;
        let all_exc = va >= exc_lo && next <= exc_hi && !overlaps_text;
        if all_text {
            l2_user_slot(i).write(l2_block(va, true, false));
        } else if all_exc {
            l2_user_slot(i).write(l2_block(va, false, true));
        } else {
            let Some(l3) = alloc_user_l3() else {
                return false;
            };
            for p in 0..512 {
                let page_va = va + p as u64 * PAGE;
                if page_va >= KERNEL_TEXT && page_va < data {
                    l3_write(l3, p, l3_page_flags(page_va, true, false));
                } else if page_va >= exc_lo && page_va < exc_hi {
                    l3_write(l3, p, l3_page_flags(page_va, false, true));
                } else {
                    l3_write(l3, p, 0);
                }
            }
            l2_user_slot(i).write(table_desc(l3));
        }
        va = next;
    }
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

/// Clean+invalidate one VA to the PoC so an aliased identity read sees it.
fn dcache_civac(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc civac, {x}",
            x = in(reg) va,
            options(nostack, preserves_flags),
        );
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
    let user_ok;
    unsafe {
        addr_of_mut!(L1.entries).write([0; 512]);
        addr_of_mut!(L2.entries).write([0; 512]);
        addr_of_mut!(L3.entries).write([0; 512]);
        addr_of_mut!(L2_RAM.entries).write([0; 512]);
        L3_KERNEL_USED = 0;
        L3_USER_USED = 0;
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
        user_ok = fill_user_map();
        // TTBR1: empty L3 behind a live table walk (ADR-016). Probe maps
        // TTBR1_PRIV later. L1[1] aliases identity RAM so EL1 can fetch
        // .text at high VA (ADR-017). Same L2_RAM: unmap identity = unmap
        // the alias. Full teardown needs split tables (Planned).
        addr_of_mut!(L1_HIGH.entries).write([0; 512]);
        addr_of_mut!(L2_HIGH.entries).write([0; 512]);
        addr_of_mut!(L3_HIGH.entries).write([0; 512]);
        l1_high_slot(0).write(table_desc(l2_high_pa()));
        l2_high_slot(0).write(table_desc(l3_high_pa()));
        l1_high_slot(1).write(table_desc(l2_ram_pa()));
    }
    dsb_ish();

    let tcr = TCR_T0SZ
        | TCR_IRGN0_WBWA
        | TCR_ORGN0_WBWA
        | TCR_SH0_INNER
        | TCR_T1SZ
        | TCR_IRGN1_WBWA
        | TCR_ORGN1_WBWA
        | TCR_SH1_INNER
        | TCR_IPS_40;
    let ttbr = l1_pa();
    let ttbr1 = l1_high_pa();
    unsafe {
        core::arch::asm!(
            "msr mair_el1, {mair}",
            "msr tcr_el1, {tcr}",
            "msr ttbr0_el1, {ttbr}",
            "msr ttbr1_el1, {ttbr1}",
            "msr tpidr_el1, {ttbr}",
            "isb",
            mair = in(reg) MAIR,
            tcr = in(reg) tcr,
            ttbr = in(reg) ttbr,
            ttbr1 = in(reg) ttbr1,
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
    // Same class as the boot-delta mark: a pre-MMU `.bss` store can be
    // invisible to a later cached read (PR #20 test image; cts-ai Docker
    // hello). Publish USER_MAP_OK only after MMU + SCTLR.C.
    USER_MAP_OK.store(user_ok, core::sync::atomic::Ordering::SeqCst);
    TTBR1_OK.store(true, core::sync::atomic::Ordering::SeqCst);
    HIGH_ALIAS_OK.store(true, core::sync::atomic::Ordering::SeqCst);
    // Exception fetch from the high alias. Identity VBAR was installed
    // before the MMU; `_start` stays at 0x4008_0000 (ADR-017).
    let _ = crate::exception::install_vbar(to_high_va(crate::exception::vector_table_addr()));
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
pub fn ttbr1_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, ttbr1_el1", v = out(reg) v);
    }
    v
}

#[allow(dead_code)]
pub fn tcr_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, tcr_el1", v = out(reg) v);
    }
    v
}

/// `true` when TCR.EPD1 is clear (TTBR1 walks are live).
#[allow(dead_code)]
pub fn ttbr1_walks_enabled() -> bool {
    tcr_el1() & TCR_EPD1 == 0
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
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    map_window_desc(va, l3_page(pa))
}

/// Map-window page with nG set (ASID-tagged). Used by the isolation probe.
#[allow(dead_code)]
pub fn map_page_ng(va: u64, pa: u64) -> bool {
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    map_window_desc(va, l3_page(pa) | DESC_NG)
}

fn map_window_desc(va: u64, desc: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    let _g = TABLES.lock();
    unsafe {
        if l3_slot(i).read() & DESC_VALID != 0 {
            return false;
        }
        l3_slot(i).write(desc);
    }
    dsb_ish();
    tlbi_va(va);
    true
}

/// Clone kernel MMIO + RAM L1 slots; give ASID B its own empty map-window L3.
#[allow(dead_code)]
pub fn prepare_asid_b_tables() -> bool {
    let _g = TABLES.lock();
    unsafe {
        addr_of_mut!(L1_ASID_B.entries).write([0; 512]);
        addr_of_mut!(L2_ASID_B.entries).write([0; 512]);
        addr_of_mut!(L3_ASID_B.entries).write([0; 512]);
        let mmio = l1_slot(0).read();
        let ram = l1_slot(1).read();
        if mmio & DESC_VALID == 0 || ram & DESC_VALID == 0 {
            return false;
        }
        l1_asid_b_slot(0).write(mmio);
        l1_asid_b_slot(1).write(ram);
        l2_asid_b_slot(0).write(table_desc(l3_asid_b_pa()));
        l1_asid_b_slot(2).write(table_desc(l2_asid_b_pa()));
    }
    dsb_ish();
    true
}

/// Map `va` → `pa` in ASID B's window L3 with nG. No TLBI (caller invalidates).
#[allow(dead_code)]
pub fn map_asid_b_ng(va: u64, pa: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    let _g = TABLES.lock();
    unsafe {
        if l3_asid_b_slot(i).read() & DESC_VALID != 0 {
            return false;
        }
        l3_asid_b_slot(i).write(l3_page(pa) | DESC_NG);
    }
    dsb_ish();
    true
}

#[allow(dead_code)]
pub fn unmap_asid_b_page(va: u64) -> bool {
    let Some(i) = window_index(va) else {
        return false;
    };
    let _g = TABLES.lock();
    unsafe {
        if l3_asid_b_slot(i).read() & DESC_VALID == 0 {
            return false;
        }
        l3_asid_b_slot(i).write(0);
    }
    dsb_ish();
    true
}

/// Walk ASID-B tables. Used by the isolation `#[test_case]`.
#[allow(dead_code)]
pub fn asid_b_mapped(va: u64) -> bool {
    let _g = TABLES.lock();
    walk_leaf(l1_asid_b_pa(), va).is_some()
}

/// True when the kernel window leaf has nG (ASID-tagged).
#[allow(dead_code)]
pub fn window_is_ng(va: u64) -> bool {
    match l3_entry(va) {
        Some(d) if d & DESC_VALID != 0 => d & DESC_NG != 0,
        _ => false,
    }
}

/// Map a 4 KiB frame at `TTBR1_PRIV` (EL1 RW, EL0 none, NX). ADR-016 first cut.
#[allow(dead_code)]
pub fn map_ttbr1_priv(pa: u64) -> bool {
    if pa & (PAGE - 1) != 0 {
        return false;
    }
    let _g = TABLES.lock();
    unsafe {
        if l3_high_slot(0).read() & DESC_VALID != 0 {
            return false;
        }
        l3_high_slot(0).write(l3_page(pa));
    }
    dsb_ish();
    tlbi_va(TTBR1_PRIV);
    true
}

/// Clear the TTBR1 private page. Identity of `pa` is unchanged.
#[allow(dead_code)]
pub fn unmap_ttbr1_priv() -> bool {
    let _g = TABLES.lock();
    unsafe {
        if l3_high_slot(0).read() & DESC_VALID == 0 {
            return false;
        }
        l3_high_slot(0).write(0);
    }
    dsb_ish();
    tlbi_va(TTBR1_PRIV);
    true
}

/// High-page leaf. `Some(d)` after a map; `Some(0)` after unmap.
#[allow(dead_code)]
pub fn ttbr1_priv_entry() -> u64 {
    let _g = TABLES.lock();
    unsafe { l3_high_slot(0).read() }
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

/// Serial addresses for the next host that hits a layout miss.
#[allow(dead_code)] // hello kernel; tests walk the same symbols.
pub fn print_layout() {
    let mut w = uart::raw();
    let _ = writeln!(
        w,
        "paging: layout data={:#x} end={:#x} pool={:#x} user={}",
        data_start(),
        frame::kernel_end(),
        frame::pool_start(),
        if user_map_ready() { 1 } else { 0 }
    );
}

/// Serial proof: MMU on, alloc, map, write/read, unmap.
///
/// Does not touch an unmapped VA (that would take an unhandled abort).
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    print_layout();
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
        // Window VA and identity VA alias one PA. Clean both so a host
        // with a real/emulated D-cache cannot lose the store.
        dcache_civac(va);
        dcache_civac(pa);
        dsb_ish();
        isb();
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
        dcache_civac(va);
        dcache_civac(pa);
        dsb_ish();
        isb();
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
        user_mapped(crate::exception::exc_stack_bottom()),
        "user map must include the exception stack (handler SP_EL1)"
    );
    assert!(
        !user_mapped(data_start()),
        "user TTBR0 must omit kernel .data"
    );
    assert!(!user_mapped(crate::heap::heap_base()));
}

#[cfg(test)]
#[test_case]
fn layout_stress_crosses_first_ram_l2() {
    // Linker ratchet: `__data_start` lives in the second RAM 2 MiB so a
    // single shared L3 cannot cover both the KERNEL_TEXT straddle and
    // the data_start straddle (cts-ai Docker / nightly growth).
    assert!(
        data_start() >= first_ram_l2_end(),
        "__data_start must leave the first RAM 2 MiB"
    );
    assert!(is_executable(KERNEL_TEXT));
    assert!(is_readonly(KERNEL_TEXT));
    assert_eq!(pxn_for(data_start()), Some(true));
    assert!(is_mapped(frame::kernel_end()));
    assert!(
        crate::exception::exc_stack_top() > first_ram_l2_end(),
        "exception stack must also leave the first RAM 2 MiB"
    );
    assert!(user_map_ready());
    assert!(user_mapped(KERNEL_TEXT));
    assert!(user_mapped(crate::exception::exc_stack_bottom()));
    assert!(!user_mapped(data_start()));
}

#[cfg(test)]
#[test_case]
fn frame_allocator_ready_after_mmu() {
    // Pre-MMU `frame::init` stores can vanish after SCTLR.C (same class
    // as the PR #20 boot-delta miss on the larger test image).
    assert!(mmu_enabled());
    assert_ne!(frame::pool_start(), 0, "ALLOC must be visible after MMU");
    assert!(frame::pool_start() >= frame::kernel_end());
    let a = frame::alloc().expect("post-MMU frame");
    assert!(a >= frame::pool_start());
    frame::free(a);
}

#[cfg(test)]
#[test_case]
fn ttbr1_tables_programmed() {
    assert!(mmu_enabled());
    assert!(ttbr1_ready(), "TTBR1 tables must publish after MMU");
    assert!(ttbr1_walks_enabled(), "TCR.EPD1 must be clear (ADR-016)");
    assert_eq!(ttbr1_el1() & !0xfff, kernel_ttbr1());
    assert_eq!(tcr_el1() & 0x3f, TCR_T0SZ);
    assert_eq!((tcr_el1() >> 16) & 0x3f, 25);
}

#[cfg(test)]
#[test_case]
fn high_alias_maps_kernel_text() {
    assert!(high_alias_ready(), "TTBR1 RAM alias must publish after MMU");
    let high_text = to_high_va(KERNEL_TEXT);
    assert!(is_high_va(high_text));
    assert_eq!(high_text, KERNEL_TEXT.wrapping_add(TTBR1_BASE));
    assert!(high_mapped(high_text), "kernel text must be aliased in TTBR1");
    assert!(
        high_is_executable(high_text),
        "high kernel text must stay PXN-clear"
    );
    let high_data = to_high_va(data_start());
    assert!(high_mapped(high_data), "RAM alias includes .data");
    assert_eq!(
        high_pxn_for(high_data),
        Some(true),
        "high .data must stay PXN"
    );
    assert!(
        high_mapped(to_high_va(crate::exception::vector_table_addr())),
        "vector table must be aliased in TTBR1"
    );
}
