//! PAN capability + enable (ADR-026 / ADR-079 / ADR-080).
//!
//! Privileged Access Never is an ARMv8.1+ feature. Default QEMU smoke
//! CPU is `-cpu cortex-a76` (ADR-079): `ID_AA64MMFR1_EL1.PAN != 0` →
//! serial `pan: present`. Historical `-cpu cortex-a57` stays
//! `pan: absent` (ADR-026 / ADR-054).
//!
//! ADR-080: after the ID-field print, enable PSTATE.PAN (`MSR PAN`) and
//! prove an EL1 load of an EL0-accessible page takes a permission
//! DABORT (`pan: enabled` / `pan: el1-fault`). Fail-closed if the fault
//! does not land. Do not claim “EL0 isolated.”
//!
//! Target features include `+pan` so the assembler accepts the `pan`
//! PSTATE name. Syscall `copy_user` / `copy_to_user` clear PAN around
//! intentional EL0-accessible copies (`with_user_access`).

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::exception;
use crate::frame;
use crate::paging;
use crate::uart;

/// `ID_AA64MMFR1_EL1` PAN field (bits [23:20]). 0 = unimplemented.
const PAN_SHIFT: u64 = 20;
const PAN_MASK: u64 = 0xf;

/// Dedicated map-window VA for the EL1-vs-EL0 fault probe (ADR-080).
/// Free of EL0_PAGE / ASID / loader-stack slots.
const PAN_PROBE_VA: u64 = paging::MAP_WINDOW + 3 * 4096;

static PAN_ID: AtomicU64 = AtomicU64::new(u64::MAX);
static PAN_PROBE_OK: AtomicBool = AtomicBool::new(false);
static PAN_ENABLED: AtomicBool = AtomicBool::new(false);
static PAN_EL1_FAULT_OK: AtomicBool = AtomicBool::new(false);

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
    PAN_PROBE_OK.load(Ordering::SeqCst)
}

/// Last published PAN ID field (u64::MAX until `observe_probe`).
#[allow(dead_code)]
pub fn published_pan_id() -> u64 {
    PAN_ID.load(Ordering::SeqCst)
}

/// True after ADR-080 left PSTATE.PAN set (enable + fault passed).
#[allow(dead_code)]
pub fn pan_is_enabled() -> bool {
    PAN_ENABLED.load(Ordering::SeqCst)
}

/// True after the EL1-vs-EL0 permission fault was caught.
#[allow(dead_code)]
pub fn pan_el1_fault_ok() -> bool {
    PAN_EL1_FAULT_OK.load(Ordering::SeqCst)
}

fn set_pstate_pan(on: bool) {
    unsafe {
        if on {
            core::arch::asm!("msr pan, #1", "isb", options(nostack, preserves_flags));
        } else {
            core::arch::asm!("msr pan, #0", "isb", options(nostack, preserves_flags));
        }
    }
}

/// Temporarily clear PSTATE.PAN for intentional EL1 access to
/// EL0-accessible pages (syscall `copy_user` / `copy_to_user`).
/// Restores PAN only if this guest left it enabled (ADR-080).
pub fn with_user_access<R>(f: impl FnOnce() -> R) -> R {
    if !PAN_ENABLED.load(Ordering::SeqCst) {
        return f();
    }
    set_pstate_pan(false);
    let r = f();
    set_pstate_pan(true);
    r
}

/// Serial proof: the probe CPU's PAN ID field. Does not `MSR PAN`.
#[allow(dead_code)] // hello kernel + `#[test_case]`.
pub fn observe_probe() -> bool {
    let id = pan_id();
    PAN_ID.store(id, Ordering::SeqCst);
    let mut w = uart::raw();
    let _ = writeln!(w, "pan: id={}", id);
    if id != 0 {
        uart::write_str_raw("pan: present\n");
        PAN_PROBE_OK.store(true, Ordering::SeqCst);
        return true;
    }
    uart::write_str_raw("pan: absent\n");
    PAN_PROBE_OK.store(true, Ordering::SeqCst);
    true
}

/// ADR-080: enable PSTATE.PAN and prove EL1 load of an EL0-accessible
/// page faults. Requires `frame::init`. Leave PAN set on success.
/// Fail-closed (no `pan: enabled` kept) if FEAT_PAN is absent or the
/// fault does not land with SPSR.PAN set.
#[allow(dead_code)] // hello kernel + `#[test_case]`.
pub fn observe_enable() -> bool {
    let id = pan_id();
    if id == 0 || !paging::mmu_enabled() {
        return false;
    }
    let Some(pa) = frame::alloc() else {
        return false;
    };
    // Plant a known word via the kernel frame alias (not EL0-accessible).
    let cpu = paging::frame_cpu_va(pa);
    unsafe {
        core::ptr::write_volatile(cpu as *mut u64, 0x5041_4e45_4c31u64);
    }
    if !paging::map_el0_ro(PAN_PROBE_VA, pa) {
        frame::free(pa);
        return false;
    }
    // Sanity: with PAN clear, EL1 may load an EL0-accessible page.
    set_pstate_pan(false);
    let pre = unsafe { core::ptr::read_volatile(PAN_PROBE_VA as *const u64) };
    if pre != 0x5041_4e45_4c31u64 {
        let _ = paging::unmap_page(PAN_PROBE_VA);
        frame::free(pa);
        return false;
    }

    set_pstate_pan(true);
    uart::write_str_raw("pan: enabled\n");

    exception::arm_pan_el1_fault();
    let ptr = black_box(PAN_PROBE_VA);
    let mut loaded: u64 = 0xDEAD_BEEF_DEAD_BEEFu64;
    unsafe {
        core::arch::asm!(
            "ldr {val}, [{p}]",
            p = in(reg) ptr,
            val = inout(reg) loaded,
            options(nostack),
        );
    }
    let caught = exception::pan_el1_fault_caught();
    let spsr_pan = exception::pan_el1_fault_spsr_pan();
    // Fail-closed: must take permission DABORT with SPSR.PAN set, and the
    // LDR must not have observed the user word.
    let ok = caught && spsr_pan && loaded == 0xDEAD_BEEF_DEAD_BEEFu64;

    let _ = paging::unmap_page(PAN_PROBE_VA);
    frame::free(pa);

    if !ok {
        set_pstate_pan(false);
        PAN_ENABLED.store(false, Ordering::SeqCst);
        PAN_EL1_FAULT_OK.store(false, Ordering::SeqCst);
        return false;
    }

    // Leave PSTATE.PAN set. Syscall uaccess clears it around copies.
    PAN_ENABLED.store(true, Ordering::SeqCst);
    PAN_EL1_FAULT_OK.store(true, Ordering::SeqCst);
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
    let pub_id = published_pan_id();
    if pub_id != u64::MAX {
        assert_eq!(pub_id, id);
    }
}

#[cfg(test)]
#[test_case]
fn pan_enable_el1_fault_on_probe_cpu() {
    assert_ne!(pan_id(), 0, "FEAT_PAN required for enable (ADR-080)");
    assert!(
        pan_is_enabled() || observe_enable(),
        "PSTATE.PAN must enable and EL1-vs-EL0 load must fault"
    );
    assert!(pan_is_enabled(), "PAN must stay enabled after ADR-080 probe");
    assert!(
        pan_el1_fault_ok(),
        "EL1 load of EL0-accessible page must have faulted"
    );
}
