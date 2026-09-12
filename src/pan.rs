//! PAN capability probe (ADR-026).
//!
//! Privileged Access Never is an ARMv8.1 feature. The default QEMU
//! line is `-cpu cortex-a57` (ARMv8.0). This module **reads**
//! `ID_AA64MMFR1_EL1.PAN` and prints the field. It does **not**
//! `MSR PAN` and does **not** change `-cpu`.
//!
//! On this probe CPU the field is 0 (`pan: absent`). Enabling PAN
//! plus an EL1-vs-EL0 access fault stays **Planned**. Do not claim
//! PAN. Do not claim “EL0 isolated.”

use core::fmt::Write;

use crate::uart;

/// `ID_AA64MMFR1_EL1` PAN field (bits [23:20]). 0 = unimplemented.
const PAN_SHIFT: u64 = 20;
const PAN_MASK: u64 = 0xf;

static PAN_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(u64::MAX);
static PAN_ABSENT_OK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

fn id_aa64mmfr1_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, id_aa64mmfr1_el1", v = out(reg) v);
    }
    v
}

/// `ID_AA64MMFR1_EL1.PAN`. 0 on `-cpu cortex-a57`.
pub fn pan_id() -> u64 {
    (id_aa64mmfr1_el1() >> PAN_SHIFT) & PAN_MASK
}

/// True when this guest published `pan: absent` (field == 0).
#[allow(dead_code)]
pub fn pan_absent_ready() -> bool {
    PAN_ABSENT_OK.load(core::sync::atomic::Ordering::SeqCst)
}

/// Last published PAN ID field (u64::MAX until `observe_probe`).
#[allow(dead_code)]
pub fn published_pan_id() -> u64 {
    PAN_ID.load(core::sync::atomic::Ordering::SeqCst)
}

/// Serial proof: the probe CPU's PAN ID field. Never enables PSTATE.PAN.
#[allow(dead_code)] // hello kernel + `#[test_case]`.
pub fn observe_probe() -> bool {
    let id = pan_id();
    PAN_ID.store(id, core::sync::atomic::Ordering::SeqCst);
    let mut w = uart::raw();
    let _ = writeln!(w, "pan: id={}", id);
    if id != 0 {
        // Present on this CPU — still do not enable here. A later cut
        // that `MSR PAN` needs its own EL1-vs-EL0 fault probe.
        uart::write_str_raw("pan: present\n");
        return false;
    }
    uart::write_str_raw("pan: absent\n");
    PAN_ABSENT_OK.store(true, core::sync::atomic::Ordering::SeqCst);
    true
}

#[cfg(test)]
#[test_case]
fn pan_unimplemented_on_probe_cpu() {
    let id = pan_id();
    assert_eq!(id, 0, "default -cpu cortex-a57 must not implement PAN");
    assert!(
        pan_absent_ready() || id == 0,
        "PAN ID field is 0; enable stays Planned"
    );
    assert_eq!(published_pan_id(), 0);
}
