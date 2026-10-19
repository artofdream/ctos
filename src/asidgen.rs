//! ADR-104 (S4): ASID allocator with a generation counter (`smp2-probe`).
//!
//! 8-bit ASIDs (TCR_EL1.AS = 0). ASIDs 0 (kernel), 1 and 2 (the fixed
//! ADR-013 / ADR-096 probe ASIDs) are never issued. A task holds a
//! [`Slot`]: `(generation, asid)`. ASIDs are never freed one by one; a
//! retired task's tables may be torn down without TLBI because its ASID is
//! not reissued until the next **rollover**: when the bitmap is exhausted the
//! allocator bumps the generation, clears the bitmap, keeps every core's
//! *running* ASID reserved (so a task live on another core never shares its
//! ASID with a new task), and issues `tlbi vmalle1is` (Inner Shareable: both
//! cores' TLBs) before any ASID is reissued.
//!
//! [`switch_to`] revalidates a slot: same generation → keep; older
//! generation but still this core's reserved running ASID → keep under the
//! new generation; otherwise allocate a fresh one. Each core records the
//! generation it last switched under in its `PerCpu::asid_gen_seen`.
//!
//! Negative build `asid-rollover-local-probe`: the rollover flush is the
//! local `tlbi vmalle1` (this core only). The cross-core reissue probe in
//! `smp.rs` must then see core 1 read a page through a stale entry.
//!
//! Not wired into the single-core default build. No scheduler: S4 runs one
//! task per core at a time.

use core::sync::atomic::Ordering;

use spin::Mutex;

/// First issuable ASID (0 kernel; 1, 2 fixed probe ASIDs).
pub const FIRST: u64 = 3;
/// Last issuable ASID (8-bit ASIDs).
pub const LAST: u64 = 255;
/// Issuable ASIDs per generation.
pub const PER_GEN: u64 = LAST - FIRST + 1;

/// A task's ASID tag. `gen == 0` means never allocated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slot {
    pub gen: u64,
    pub asid: u64,
}

impl Slot {
    pub const NONE: Slot = Slot { gen: 0, asid: 0 };
}

struct State {
    gen: u64,
    bitmap: [u64; 4],
    next: u64,
    /// ASID running on each core (0 = none / kernel).
    active: [u64; crate::percpu::NCPU],
    rollovers: u64,
}

static STATE: Mutex<State> = Mutex::new(State {
    gen: 1,
    bitmap: [0; 4],
    next: FIRST,
    active: [0; crate::percpu::NCPU],
    rollovers: 0,
});

fn test_bit(b: &[u64; 4], a: u64) -> bool {
    b[(a / 64) as usize] & (1 << (a % 64)) != 0
}

fn set_bit(b: &mut [u64; 4], a: u64) {
    b[(a / 64) as usize] |= 1 << (a % 64);
}

/// Broadcast flush of every non-global entry on every core (Inner Shareable).
fn flush_all_cores() {
    unsafe {
        #[cfg(not(feature = "asid-rollover-local-probe"))]
        core::arch::asm!("dsb ishst", "tlbi vmalle1is", "dsb ish", "isb", options(nostack, preserves_flags));
        #[cfg(feature = "asid-rollover-local-probe")]
        core::arch::asm!("dsb ishst", "tlbi vmalle1", "dsb ish", "isb", options(nostack, preserves_flags));
    }
}

fn rollover(s: &mut State) {
    s.gen += 1;
    s.rollovers += 1;
    s.bitmap = [0; 4];
    // Running ASIDs stay reserved into the new generation.
    for &a in s.active.iter() {
        if a >= FIRST {
            set_bit(&mut s.bitmap, a);
        }
    }
    s.next = FIRST;
    flush_all_cores();
}

fn alloc_locked(s: &mut State) -> Slot {
    loop {
        while s.next <= LAST {
            let a = s.next;
            s.next += 1;
            if !test_bit(&s.bitmap, a) {
                set_bit(&mut s.bitmap, a);
                return Slot { gen: s.gen, asid: a };
            }
        }
        rollover(s);
    }
}

/// Allocate a fresh `(gen, asid)`; may roll over (broadcast flush first).
pub fn alloc() -> Slot {
    alloc_locked(&mut STATE.lock())
}

/// Rollovers so far.
pub fn rollovers() -> u64 {
    STATE.lock().rollovers
}

/// Revalidate `slot` for this core and mark its ASID running here. Returns
/// the ASID to program into TTBR0. Records the generation in this core's
/// `PerCpu::asid_gen_seen`.
pub fn switch_to(slot: &mut Slot) -> u64 {
    let cpu = crate::percpu::id();
    let mut s = STATE.lock();
    if slot.gen != s.gen {
        // Reserved across the rollover because it was running on this core.
        if slot.gen != 0 && s.active[cpu] == slot.asid && slot.asid >= FIRST {
            slot.gen = s.gen;
        } else {
            *slot = alloc_locked(&mut s);
        }
    }
    s.active[cpu] = slot.asid;
    let gen = s.gen;
    drop(s);
    if let Some(b) = crate::percpu::this() {
        b.asid_gen_seen.store(gen, Ordering::SeqCst);
    }
    slot.asid
}

/// This core no longer runs a task ASID (back on the kernel ASID 0).
pub fn release_cpu() {
    let cpu = crate::percpu::id();
    STATE.lock().active[cpu] = 0;
}
