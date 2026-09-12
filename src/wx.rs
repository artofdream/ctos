//! W^X probe: heap NX + kernel text executable (NFR-10 / ADR-012).
//!
//! Cooperative worker stacks are heap `Vec`s, so heap PXN covers them.
//! Live linker stacks are RW+NX with `.data` (ADR-015). This module is
//! still the heap execute-from probe, not a “secure OS” claim.

use alloc::boxed::Box;
use core::fmt::Write;
use core::hint::black_box;

use crate::exception;
use crate::heap;
use crate::paging;
use crate::uart;

/// AArch64 `RET` (`D65F03C0`). Only runs if PXN failed (probe Failed).
const RET_A64: u32 = 0xD65F03C0;

fn flags_ok() -> bool {
    if !paging::is_executable(paging::KERNEL_TEXT) {
        return false;
    }
    if paging::image_pxn_for(heap::heap_base()) != Some(true) {
        return false;
    }
    if paging::image_pxn_for(heap::heap_end() - 1) != Some(true) {
        return false;
    }
    // Device MMIO L1 stays XN.
    paging::pxn_for(0x0900_0000) == Some(true)
}

fn sync_icache(ptr: *const u32) {
    let x = ptr as u64;
    unsafe {
        core::arch::asm!(
            "dc cvau, {x}",
            "dsb ish",
            "ic ivau, {x}",
            "dsb ish",
            "isb",
            x = in(reg) x,
            options(nostack, preserves_flags),
        );
    }
}

fn execute_from_heap_caught() -> bool {
    let mut buf = Box::new([0u32; 2]);
    buf[0] = RET_A64;
    buf[1] = RET_A64;
    let ptr = buf.as_ptr();
    sync_icache(ptr);
    exception::arm_nx_probe();
    let f: extern "C" fn() = unsafe { core::mem::transmute(black_box(ptr)) };
    black_box(f)();
    exception::nx_probe_caught()
}

/// Serial proof: page flags + a caught execute-from-heap IABORT.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !heap::is_ready() || !flags_ok() {
        return false;
    }
    if !execute_from_heap_caught() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "wx: ok");
    true
}

#[cfg(test)]
#[test_case]
fn execute_from_heap_is_caught() {
    assert!(heap::is_ready());
    assert!(flags_ok(), "PXN flags missing on heap or present on text");
    assert!(
        execute_from_heap_caught(),
        "execute-from-heap must take a permission IABORT"
    );
}
