//! ARM generic physical timer (CNTP) for QEMU `virt` (FR-08 / M5).
//!
//! PPI 30 is the non-secure EL1 physical timer. Ticks are counted and the
//! first one prints `timer: tick` on the raw PL011. Not a board timer.
//! See ADR-006.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::gic;
use crate::uart;

/// GIC PPI for the non-secure EL1 physical timer (ARM GIC architecture).
pub const PPI: u32 = 30;

/// Wait this many counter ticks (~2s at a 62.5 MHz CNTFRQ) before failing.
const OBSERVE_TIMEOUT_DIV: u64 = 1;
const OBSERVE_TIMEOUT_MUL: u64 = 2;
/// Aim for ~10 ms between firings (`CNTFRQ / 100`).
const INTERVAL_DIV: u64 = 100;
const INTERVAL_MIN: u64 = 1000;

static TICKS: AtomicU64 = AtomicU64::new(0);
static INTERVAL: AtomicU64 = AtomicU64::new(INTERVAL_MIN);

/// IRQ-to-handler CNTPCT−CVAL samples (NFR-07). Not a latency budget.
const IRQ_DELTA_CAP: usize = 16;
static IRQ_DELTAS: [AtomicU64; IRQ_DELTA_CAP] = [const { AtomicU64::new(0) }; IRQ_DELTA_CAP];
static IRQ_DELTA_N: AtomicU64 = AtomicU64::new(0);

fn cntfrq() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq);
    }
    freq
}

#[allow(dead_code)] // hello + `#[test_case]` + `src/perf.rs`.
pub fn cntpct() -> u64 {
    let t: u64;
    unsafe {
        core::arch::asm!("mrs {t}, cntpct_el0", t = out(reg) t);
    }
    t
}

fn write_ctl(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr cntp_ctl_el0, {v}",
            "isb",
            v = in(reg) value,
        );
    }
}

fn write_tval(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr cntp_tval_el0, {v}",
            "isb",
            v = in(reg) value,
        );
    }
}

fn cntp_cval() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, cntp_cval_el0", v = out(reg) v);
    }
    v
}

fn reset_irq_deltas() {
    IRQ_DELTA_N.store(0, Ordering::SeqCst);
    for d in IRQ_DELTAS.iter() {
        d.store(0, Ordering::SeqCst);
    }
}

fn record_irq_delta(delta: u64) {
    let n = IRQ_DELTA_N.fetch_add(1, Ordering::SeqCst);
    if (n as usize) < IRQ_DELTA_CAP {
        IRQ_DELTAS[n as usize].store(delta, Ordering::SeqCst);
    }
}

/// `(min, max, n)` of CNTPCT−CVAL samples captured in `on_interrupt`.
#[allow(dead_code)] // hello perf probe + `#[test_case]`.
pub fn irq_delta_stats() -> Option<(u64, u64, u64)> {
    let n = IRQ_DELTA_N.load(Ordering::SeqCst).min(IRQ_DELTA_CAP as u64);
    if n == 0 {
        return None;
    }
    let mut min = u64::MAX;
    let mut max = 0u64;
    for i in 0..n as usize {
        let d = IRQ_DELTAS[i].load(Ordering::SeqCst);
        min = min.min(d);
        max = max.max(d);
    }
    Some((min, max, n))
}

fn interval() -> u64 {
    INTERVAL.load(Ordering::Relaxed)
}

/// Program CNTP and enable its PPI. Leaves DAIF.I masked.
pub fn init() {
    let freq = cntfrq();
    let iv = if freq == 0 {
        INTERVAL_MIN
    } else {
        freq.saturating_div(INTERVAL_DIV).max(INTERVAL_MIN)
    };
    INTERVAL.store(iv, Ordering::Relaxed);
    gic::enable_ppi(PPI);
    write_ctl(0);
    write_tval(iv);
    write_ctl(1);
}

pub fn stop() {
    write_ctl(0);
}

/// Rearm CNTP with a short TVAL so an EL0 `WFI` can take a lower-EL IRQ (ADR-040).
pub fn arm_soon() {
    let iv = interval().saturating_div(10).max(INTERVAL_MIN);
    write_ctl(0);
    write_tval(iv);
    write_ctl(1);
}

pub fn tick_count() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

/// Rearm and count. First tick is the serial marker for qemu-smoke.
/// Records CNTPCT−CVAL before rearm (IRQ-to-handler delta).
pub fn on_interrupt() {
    let now = cntpct();
    let cval = cntp_cval();
    let delta = if now >= cval { now - cval } else { 0 };
    record_irq_delta(delta);
    write_tval(interval());
    let n = TICKS.fetch_add(1, Ordering::SeqCst);
    if n == 0 {
        uart::write_str_raw("timer: tick\n");
    }
}

/// Unmask IRQs until `n` ticks arrive or the physical counter times out.
pub fn observe_ticks(n: u64) -> bool {
    reset_irq_deltas();
    write_ctl(0);
    write_tval(interval());
    write_ctl(1);
    let start = tick_count();
    let freq = cntfrq();
    let timeout = if freq == 0 {
        125_000_000
    } else {
        freq.saturating_mul(OBSERVE_TIMEOUT_MUL) / OBSERVE_TIMEOUT_DIV
    };
    let t0 = cntpct();
    gic::unmask_irqs();
    while tick_count() < start.saturating_add(n) {
        if cntpct().wrapping_sub(t0) > timeout {
            gic::mask_irqs();
            stop();
            return false;
        }
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
    gic::mask_irqs();
    stop();
    true
}

#[allow(dead_code)] // `#[test_case]`.
pub fn freq() -> u64 {
    cntfrq()
}

#[cfg(test)]
#[test_case]
fn cntfrq_is_nonzero() {
    assert_ne!(freq(), 0);
}

#[cfg(test)]
#[test_case]
fn timer_tick_is_observable() {
    let before = tick_count();
    assert!(observe_ticks(1), "CNTP PPI 30 did not fire");
    assert!(tick_count() > before);
}

#[cfg(test)]
#[test_case]
fn irq_delta_samples_recorded() {
    assert!(observe_ticks(4), "need several CNTP firings for irq-delta");
    let (min, max, n) = irq_delta_stats().expect("no IRQ-to-handler samples");
    assert!(n >= 4, "expected at least 4 samples, got {n}");
    assert!(max >= min);
}
