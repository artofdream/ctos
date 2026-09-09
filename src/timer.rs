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

fn cntfrq() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!("mrs {f}, cntfrq_el0", f = out(reg) freq);
    }
    freq
}

fn cntpct() -> u64 {
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

pub fn tick_count() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

/// Rearm and count. First tick is the serial marker for qemu-smoke.
pub fn on_interrupt() {
    write_tval(interval());
    let n = TICKS.fetch_add(1, Ordering::SeqCst);
    if n == 0 {
        uart::write_str_raw("timer: tick\n");
    }
}

/// Unmask IRQs until `n` ticks arrive or the physical counter times out.
pub fn observe_ticks(n: u64) -> bool {
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
