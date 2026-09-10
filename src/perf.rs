//! CNTPCT probes (NFR-07 / ADR-011): loop baseline + IRQ-to-handler delta.
//!
//! The loop probe proves the physical counter is readable and advances.
//! The IRQ probe records CNTPCT−CVAL when the timer handler runs (min /
//! max / spread). Neither is a published bench or a latency budget.
//! See `docs/framework/performance.md`.

use core::fmt::Write;
use core::hint::black_box;

use crate::timer;
use crate::uart;

/// Enough iterations that QEMU virt CNTPCT should move. Not a workload.
const LOOP_ITERS: u64 = 10_000;

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
