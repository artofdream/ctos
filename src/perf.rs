//! CNTPCT probes (NFR-07 / NFR-08 / ADR-011): loop baseline, IRQ-to-handler
//! delta, and boot-to-ready.
//!
//! The loop probe proves the physical counter is readable and advances.
//! The IRQ probe records CNTPCT−CVAL when the timer handler runs (min /
//! max / spread). The boot probe is CNTPCT from `kernel_main` entry to
//! after init / `Hello World!`. None is a published bench or a latency
//! budget. See `docs/framework/performance.md`.

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::timer;
use crate::uart;

static BOOT_EARLY: AtomicU64 = AtomicU64::new(0);
static BOOT_DELTA: AtomicU64 = AtomicU64::new(0);
static BOOT_MARKED: AtomicBool = AtomicBool::new(false);

/// Enough iterations that QEMU virt CNTPCT should move. Not a workload.
const LOOP_ITERS: u64 = 10_000;

/// First instruction of `kernel_main` (post-BSS). Not `_start`.
pub fn mark_early() {
    BOOT_EARLY.store(timer::cntpct(), Ordering::SeqCst);
    BOOT_MARKED.store(true, Ordering::SeqCst);
}

/// After init markers (`Hello World!`). Stores the boot-to-ready delta.
pub fn mark_ready() -> bool {
    if !BOOT_MARKED.load(Ordering::SeqCst) {
        return false;
    }
    let t0 = BOOT_EARLY.load(Ordering::SeqCst);
    let delta = timer::cntpct().wrapping_sub(t0);
    if delta == 0 {
        return false;
    }
    BOOT_DELTA.store(delta, Ordering::SeqCst);
    true
}

#[allow(dead_code)]
pub fn boot_delta() -> Option<u64> {
    let d = BOOT_DELTA.load(Ordering::SeqCst);
    if d == 0 {
        None
    } else {
        Some(d)
    }
}

/// Serial proof: CNTPCT advanced from `kernel_main` entry to ready.
#[allow(dead_code)]
pub fn observe_boot_delta() -> bool {
    let Some(delta) = boot_delta() else {
        return false;
    };
    let mut w = uart::raw();
    let _ = writeln!(w, "perf: boot-delta ticks={delta}");
    true
}

/// Serial proof: `CNTFRQ != 0` and `CNTPCT` advances. Prints the delta.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if timer::freq() == 0 {
        return false;
    }
    let Some(delta) = loop_delta() else {
        return false;
    };
    let mut w = uart::raw();
    let _ = writeln!(w, "perf: cntpct delta={delta}");
    true
}

fn loop_delta() -> Option<u64> {
    let t0 = timer::cntpct();
    let mut acc: u64 = 0;
    let mut i: u64 = 0;
    while i < LOOP_ITERS {
        acc = black_box(acc.wrapping_add(i));
        i = black_box(i.wrapping_add(1));
    }
    let _ = black_box(acc);
    let delta = timer::cntpct().wrapping_sub(t0);
    if delta == 0 {
        None
    } else {
        Some(delta)
    }
}

/// Serial proof: IRQ-to-handler CNTPCT−CVAL min/max/spread after ticks.
///
/// Call after `timer::observe_ticks` so samples already exist. Does not
/// start a second IRQ window (the timer is stopped after observe).
#[allow(dead_code)] // hello kernel only; cargo test uses the timer case.
pub fn observe_irq_delta() -> bool {
    let Some((min, max, n)) = timer::irq_delta_stats() else {
        return false;
    };
    if n == 0 || max < min {
        return false;
    }
    let spread = max - min;
    let mut w = uart::raw();
    let _ = writeln!(w, "perf: irq-delta min={min} max={max} spread={spread} n={n}");
    true
}

#[cfg(test)]
#[test_case]
fn cntpct_advances_over_loop() {
    assert_ne!(timer::freq(), 0, "CNTFRQ_EL0 is zero");
    let delta = loop_delta().expect("CNTPCT_EL0 did not advance");
    assert!(delta > 0);
}

#[cfg(test)]
#[test_case]
fn boot_delta_sample_exists() {
    let delta = boot_delta().expect("perf: boot-delta sample missing");
    assert!(delta > 0);
}
