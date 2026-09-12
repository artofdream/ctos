//! Freestanding `libctos` hello payload (Track A / A2 / ADR-022).
//!
//! The kernel copies a host-built image (linked against `libctos`) onto
//! `paging::EL0_PAGE` and `ERET`s to it — same standing-EL0 style as A1.
//! Leftover mile (ADR-032): bytes come from FAT `/hello` (flatten
//! `PT_LOAD`), not `include_bytes!`. Still not an ELF loader (A3).
//! Not app hosting. Not POSIX.

use alloc::vec::Vec;
use core::fmt::Write;
use core::hint::black_box;

use crate::el0;
use crate::exception;
use crate::frame;
use crate::loader;
use crate::paging;
use crate::syscall;
use crate::uart;

include!(concat!(env!("OUT_DIR"), "/hello_libctos_meta.rs"));
const _: () = assert!(HELLO_ELF_LEN > HELLO_LEN);
const _: () = assert!(HELLO_LOAD_VA == paging::EL0_PAGE);
const _: () = assert!(HELLO_LEN > 0);
const _: () = assert!(HELLO_LEN <= 3584);

fn hello_bin() -> Option<Vec<u8>> {
    let elf = loader::hello_elf()?;
    let image = loader::flatten_pt_load(&elf)?;
    if image.len() != HELLO_LEN {
        return None;
    }
    Some(image)
}

fn sync_range(ptr: *const u8, len: usize) {
    let mut x = ptr as u64;
    let end = x + len as u64;
    while x < end {
        unsafe {
            core::arch::asm!(
                "dc cvau, {x}",
                "ic ivau, {x}",
                x = in(reg) x,
                options(nostack, preserves_flags),
            );
        }
        x += 64;
    }
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

fn run_hello() -> bool {
    if el0::is_active() || !paging::user_map_ready() {
        return false;
    }
    syscall::reset_probe_flags();
    let Some(code_pa) = frame::alloc() else {
        return false;
    };
    let Some(stack_pa) = frame::alloc() else {
        frame::free(code_pa);
        return false;
    };
    let va = paging::EL0_PAGE;
    let stack_va = va + 4096;
    if !paging::map_el0_exec(va, code_pa) {
        frame::free(code_pa);
        frame::free(stack_pa);
        return false;
    }
    if !paging::map_el0_rw(stack_va, stack_pa) {
        let _ = paging::unmap_page(va);
        frame::free(code_pa);
        frame::free(stack_pa);
        return false;
    }
    let Some(bin) = hello_bin() else {
        let _ = paging::unmap_page(stack_va);
        let _ = paging::unmap_page(va);
        frame::free(stack_pa);
        frame::free(code_pa);
        return false;
    };
    unsafe {
        core::ptr::copy_nonoverlapping(bin.as_ptr(), va as *mut u8, bin.len());
    }
    sync_range(va as *const u8, bin.len());
    // Rust `main` saves x30 on SP. The code page is EL0-exec / no EL0 data
    // (and WXN forbids making it W+X). Stack is a second EL0-RW NX page.
    let user_sp = stack_va + 4096;
    el0::install_standing(va, user_sp, paging::user_ttbr0());
    unsafe {
        exception::eret_to_el0(black_box(va), 0, user_sp);
    }
    let _ = paging::unmap_page(stack_va);
    let _ = paging::unmap_page(va);
    frame::free(stack_pa);
    frame::free(code_pa);
    if el0::is_active() {
        el0::clear_active();
        return false;
    }
    syscall::yield_seen()
        && syscall::uart_seen()
        && syscall::exit_seen()
        && syscall::last_uart_write() == 12
        && syscall::last_exit_status() == 0
        && syscall::yield_count() >= 1
}

/// Serial proof: linked libctos hello printed via uart_write. Not app hosting.
#[allow(dead_code)] // hello kernel only; cargo test uses the case below.
pub fn observe_probe() -> bool {
    if !paging::mmu_enabled() || !paging::user_map_ready() {
        return false;
    }
    if !run_hello() {
        return false;
    }
    let mut w = uart::raw();
    let _ = writeln!(w, "libctos: linked");
    true
}

#[cfg(test)]
#[test_case]
fn libctos_numbers_match_a1_abi() {
    assert_eq!(syscall::SYS_EXIT, 16);
    assert_eq!(syscall::SYS_UART_WRITE, 17);
    assert_eq!(syscall::SYS_YIELD, 18);
    assert_eq!(syscall::SYS_FS_CREATE, 19);
    assert_eq!(syscall::SYS_FS_OPEN, 20);
    assert_eq!(syscall::SYS_FS_READ, 21);
    assert_eq!(syscall::SYS_FS_WRITE, 22);
    assert_eq!(syscall::SYS_FS_CLOSE, 23);
    assert_ne!(syscall::SYS_EXIT, syscall::SVC_PROBE_RETURN);
    assert_ne!(syscall::SYS_EXIT, syscall::SVC_PROBE_STANDING);
    assert_ne!(syscall::SYS_EXIT, syscall::SVC_PROBE_RESTORE);
}

#[cfg(test)]
#[test_case]
fn libctos_payload_encodes_public_svc_only() {
    let bin = hello_bin().expect("FAT /hello flatten must match this build");
    assert_eq!(bin.len(), HELLO_LEN);
    let svc = |imm: u32| 0xD4000001u32 | (imm << 5);
    let words: &[u32] = unsafe {
        core::slice::from_raw_parts(bin.as_ptr().cast::<u32>(), bin.len() / 4)
    };
    assert!(
        words.contains(&svc(16)) && words.contains(&svc(17)) && words.contains(&svc(18)),
        "FAT payload must issue SVC #16/#17/#18"
    );
    assert!(
        !words.contains(&svc(0)) && !words.contains(&svc(1)) && !words.contains(&svc(2)),
        "FAT payload must not issue reserved SVC #0/#1/#2"
    );
    assert!(bin.windows(12).any(|w| w == b"libctos: hi\n"));
    assert!(bin.windows(12).any(|w| w == b"libctos: ok\n"));
}

#[cfg(test)]
#[test_case]
fn libctos_hello_yield_uart_exit() {
    assert!(paging::mmu_enabled());
    assert!(
        run_hello(),
        "linked libctos hello must uart_write, yield, then exit"
    );
    assert!(!el0::is_active());
    assert_eq!(syscall::last_uart_write(), 12);
    assert_eq!(syscall::last_exit_status(), 0);
}
