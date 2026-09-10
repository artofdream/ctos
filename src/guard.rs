//! Linker-stack guard pages (NFR-10 / ADR-014).
//!
//! Three 4 KiB holes sit below the thread / exception / fatal stacks.
//! They are unmapped after the identity tables are filled. A store into
//! the thread-stack guard must take a current-EL translation data abort.
//! The live stack pages stay executable — this is not kernel W^X.

use core::fmt::Write;
use core::hint::black_box;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::exception;
use crate::paging;
use crate::uart;

static FAULT_CAUGHT: AtomicBool = AtomicBool::new(false);

/// Called from the current-EL sync handler when a translation DABORT
/// hits a guard page while the probe is armed.
pub fn note_fault() {
    FAULT_CAUGHT.store(true, Ordering::SeqCst);
    uart::write_str_raw("guard: fault\n");
}

fn guards_unmapped() -> bool {
    for va in exception::stack_guards() {
        if paging::is_mapped(va) {
            return false;
        }
        if va & 0xfff != 0 {
            return false;
        }
    }
    exception::thread_stack_guard() + 4096 == exception::thread_stack_bottom()
        && exception::exc_stack_guard() + 4096 == exception::exc_stack_bottom()
        && exception::fatal_stack_guard() + 4096 == exception::fatal_stack_bottom()
}

fn store_to_thread_guard_caught() -> bool {
    FAULT_CAUGHT.store(false, Ordering::SeqCst);
    exception::arm_guard_probe();
    let p = black_box(exception::thread_stack_guard());
    let val = black_box(0x4755_4152u64);
    unsafe {
        core::arch::asm!(
            "str {val}, [{ptr}]",
            ptr = in(reg) p,
            val = in(reg) val,
            options(nostack),
        );
    }
    exception::disarm_guard_probe();
    FAULT_CAUGHT.load(Ordering::SeqCst)
}

/// Serial proof: guards unmapped + a caught store to the thread guard.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !guards_unmapped() {
        return false;
    }
    if !store_to_thread_guard_caught() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "guard: ok");
    true
}

#[cfg(test)]
#[test_case]
fn linker_stack_guards_unmapped() {
    assert!(paging::mmu_enabled());
    assert!(
        guards_unmapped(),
        "each linker-stack guard must be a 4 KiB unmapped hole"
    );
}

#[cfg(test)]
#[test_case]
fn store_to_thread_guard_is_caught() {
    assert!(
        store_to_thread_guard_caught(),
        "store to __stack_guard must take a translation data abort"
    );
}
