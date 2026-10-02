//! Freestanding `libctos` hello payload (Track A / A2 / ADR-022).
//!
//! The kernel copies a host-built image (linked against `libctos`) onto
//! `paging::EL0_PAGE` and `ERET`s to it — same standing-EL0 style as A1.
//! Leftover mile (ADR-032): bytes come from FAT `/hello` (flatten
//! `PT_LOAD`), not `include_bytes!`. Still not an ELF loader (A3).
//! Not app hosting. Not POSIX.
//! ADR-097: text is mapped EL0 execute-only and the app's `.rodata` is on
//! the app-hdr page (`MAP_WINDOW`, EL0 read-only), as in the A3 loader.

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

/// Text image (flattened `PT_LOAD` at `EL0_PAGE`) and, since ADR-097, the
/// app-hdr page (ELF headers + `.rodata`) that the app's strings live on.
fn hello_bin() -> Option<(Vec<u8>, Vec<u8>)> {
    let elf = loader::hello_elf()?;
    let image = loader::flatten_pt_load(&elf)?;
    if image.len() != HELLO_LEN {
        return None;
    }
    let ro = loader::ro_page_image(&elf)?;
    Some((image, ro))
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
    let Some(ro_pa) = frame::alloc() else {
        frame::free(code_pa);
        frame::free(stack_pa);
        return false;
    };
    let va = paging::EL0_PAGE;
    let stack_va = va + 4096;
    let ro_va = paging::MAP_WINDOW;
    let Some((bin, ro)) = hello_bin() else {
        frame::free(ro_pa);
        frame::free(stack_pa);
        frame::free(code_pa);
        return false;
    };
    // ADR-097: `.rodata` sits on the app-hdr page (EL0 read-only, NX); fill it
    // through the alias before mapping.
    unsafe {
        core::ptr::copy_nonoverlapping(ro.as_ptr(), paging::frame_cpu_va(ro_pa) as *mut u8, ro.len());
    }
    // ADR-097: text is EL0 execute-only (same mapping as the A3 loader).
    // Bytes go in via the frame alias.
    if !map_text(va, code_pa) {
        frame::free(ro_pa);
        frame::free(code_pa);
        frame::free(stack_pa);
        return false;
    }
    if !paging::map_el0_rw(stack_va, stack_pa) || !paging::map_el0_ro(ro_va, ro_pa) {
        let _ = paging::unmap_page(stack_va);
        let _ = paging::unmap_page(va);
        frame::free(ro_pa);
        frame::free(code_pa);
        frame::free(stack_pa);
        return false;
    }
    // EL1 sees the EL0-text VA as read-only; write through the TTBR1 alias,
    // then clean/invalidate both aliases before EL0 fetches.
    let alias = paging::frame_cpu_va(code_pa);
    unsafe {
        core::ptr::copy_nonoverlapping(bin.as_ptr(), alias as *mut u8, bin.len());
    }
    sync_range(alias as *const u8, bin.len());
    // ADR-097: the XO text VA is not EL0-data-accessible, so PAN does not
    // apply; the uaccess window stays for the `xo-leak-probe` build.
    crate::pan::with_user_access(|| sync_range(va as *const u8, bin.len()));
    // Rust `main` saves x30 on SP. The code page is EL0-exec / no EL0 data
    // (and WXN forbids making it W+X). Stack is a second EL0-RW NX page.
    let user_sp = stack_va + 4096;
    el0::install_standing(va, user_sp, paging::user_ttbr0());
    unsafe {
        exception::eret_to_el0(black_box(va), 0, user_sp);
    }
    let _ = paging::unmap_page(stack_va);
    let _ = paging::unmap_page(va);
    let _ = paging::unmap_page(ro_va);
    frame::free(stack_pa);
    frame::free(code_pa);
    frame::free(ro_pa);
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

#[cfg(not(feature = "xo-leak-probe"))]
fn map_text(va: u64, pa: u64) -> bool {
    paging::map_el0_xo(va, pa)
}

#[cfg(feature = "xo-leak-probe")]
fn map_text(va: u64, pa: u64) -> bool {
    paging::map_el0_text(va, pa)
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
    let (bin, ro) = hello_bin().expect("FAT /hello flatten must match this build");
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
    // ADR-097: the strings live in the app-hdr (rodata) page, not in text.
    for m in [b"libctos: hi\n", b"libctos: ok\n"] {
        assert!(ro.windows(12).any(|w| w == m));
        assert!(!bin.windows(12).any(|w| w == m));
    }
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
