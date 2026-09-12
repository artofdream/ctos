//! RO+NX text/data split (NFR-10 / ADR-015).
//!
//! `.text`/`.rodata` are RO+X. `.data`/`.bss` and live linker stacks are
//! RW+NX. `SCTLR.WXN` is on because text is no longer writable.
//! This is W^X of the identity *image* on this QEMU virt guest — not a
//! “secure OS” and not a claim that every future mapping stays W^X.

use core::fmt::Write;
use core::hint::black_box;

use crate::exception;
use crate::paging;
use crate::uart;

/// AArch64 `RET`. Only runs if `.data` is still executable (probe Failed).
const RET_A64: u32 = 0xD65F03C0;

#[used]
static mut DATA_BAIT: [u32; 2] = [RET_A64, RET_A64];

fn data_bait_va() -> u64 {
    core::ptr::addr_of!(DATA_BAIT) as u64
}

fn flags_ok() -> bool {
    if !paging::is_executable(paging::KERNEL_TEXT) {
        return false;
    }
    if !paging::is_readonly(paging::KERNEL_TEXT) {
        return false;
    }
    if !paging::wxn_enabled() {
        return false;
    }
    let data = data_bait_va();
    if paging::image_pxn_for(data) != Some(true) {
        return false;
    }
    if paging::image_readonly_for(data) != Some(false) {
        return false;
    }
    let stack = crate::exception::thread_stack_bottom();
    paging::image_pxn_for(stack) == Some(true)
        && paging::image_readonly_for(stack) == Some(false)
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

fn execute_from_data_caught() -> bool {
    let ptr = core::ptr::addr_of!(DATA_BAIT) as *const u32;
    sync_icache(ptr);
    exception::arm_ro_nx_probe();
    let f: extern "C" fn() = unsafe { core::mem::transmute(black_box(ptr)) };
    black_box(f)();
    exception::ro_nx_probe_caught()
}

fn write_to_text_caught() -> bool {
    exception::arm_ro_write_probe();
    let p = black_box(paging::KERNEL_TEXT);
    let val = black_box(0x524F_5752u64);
    unsafe {
        core::arch::asm!(
            "str {val}, [{ptr}]",
            ptr = in(reg) p,
            val = in(reg) val,
            options(nostack),
        );
    }
    exception::ro_write_probe_caught()
}

/// Serial proof: flags + execute-from-`.data` + write-to-RO-text.
#[allow(dead_code)] // hello kernel only; cargo test uses the cases below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !flags_ok() {
        return false;
    }
    if !execute_from_data_caught() {
        return false;
    }
    if !write_to_text_caught() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "ro: ok");
    true
}

#[cfg(test)]
#[test_case]
fn execute_from_data_is_caught() {
    assert!(flags_ok(), "RO+X / RW+NX flags missing");
    assert!(
        execute_from_data_caught(),
        "execute-from-.data must take a permission IABORT"
    );
}

#[cfg(test)]
#[test_case]
fn write_to_ro_text_is_caught() {
    assert!(paging::is_readonly(paging::KERNEL_TEXT));
    assert!(
        write_to_text_caught(),
        "store to RO text must take a permission DABORT"
    );
}
