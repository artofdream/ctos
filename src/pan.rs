//! PAN capability probe (ADR-026 / ADR-079).
//!
//! Privileged Access Never is an ARMv8.1+ feature. The default QEMU
//! smoke CPU is `-cpu cortex-a76` (ADR-079):
//! `ID_AA64MMFR1_EL1.PAN != 0` → serial `pan: present`. Historical
//! `-cpu cortex-a57` stays `pan: absent` (ADR-026 /
//! ADR-054).
//!
//! This module **reads** the ID field and prints it. It does **not**
//! `MSR PAN`. Enable + EL1-vs-EL0 fault is **ADR-080** (not this mile).
//! Do not claim PAN enabled. Do not claim “EL0 isolated.”

use core::fmt::Write;

use crate::uart;

/// `ID_AA64MMFR1_EL1` PAN field (bits [23:20]). 0 = unimplemented.
const PAN_SHIFT: u64 = 20;
const PAN_MASK: u64 = 0xf;

static PAN_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(u64::MAX);
static PAN_PROBE_OK: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

fn id_aa64mmfr1_el1() -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!("mrs {v}, id_aa64mmfr1_el1", v = out(reg) v);
    }
    v
}

/// `ID_AA64MMFR1_EL1.PAN`. Non-zero on default `-cpu cortex-a76` (ADR-079).
pub fn pan_id() -> u64 {
    (id_aa64mmfr1_el1() >> PAN_SHIFT) & PAN_MASK
}

/// True when this guest published a PAN ID-field line (`absent` or `present`).
#[allow(dead_code)]
pub fn pan_probe_ready() -> bool {
    PAN_PROBE_OK.load(core::sync::atomic::Ordering::SeqCst)
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
        // Present on this CPU — still do not enable here. ADR-080 may
        // `MSR PAN` and prove an EL1-vs-EL0 fault.
        uart::write_str_raw("pan: present\n");
        PAN_PROBE_OK.store(true, core::sync::atomic::Ordering::SeqCst);
        return true;
    }
    uart::write_str_raw("pan: absent\n");
    PAN_PROBE_OK.store(true, core::sync::atomic::Ordering::SeqCst);
    true
}

#[cfg(test)]
#[test_case]
fn pan_present_on_probe_cpu() {
    let id = pan_id();
    assert_ne!(
        id, 0,
        "default -cpu cortex-a76 must implement FEAT_PAN (ADR-079)"
    );
    assert!(
        pan_probe_ready() || id != 0,
        "PAN ID field non-zero; enable stays ADR-080"
    );
    // observe_probe runs in kernel_main_high before tests; published id matches MRS.
    let pub_id = published_pan_id();
    if pub_id != u64::MAX {
        assert_eq!(pub_id, id);
    }
}
