//! Freestanding EL0 sample that exercises FAT via thin VFS (ADR-061).
//!
//! Opens/reads the existing FAT16 `/probe` payload (`fat-hi`) through
//! libctos `fs_open`/`fs_read`/`fs_close` (SVC 20/21/23). Thin VFS routes
//! `/` → FAT16 (ADR-058). Deeper than `fs-libctos` (`/memdemo` memfs only).
//!
//! Important: EL0 exec pages are fetch-only (AP no EL0 data; see
//! `paging::l3_page_el0_exec`). String literals may be passed to SVCs (EL1
//! copies them) but must not be loaded from EL0. Compare with immediates.
//! Not POSIX. Not getdents. CRT `_start` lives in `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, fs_close, fs_open, fs_read, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

/// Existing FAT16 probe file (A7 / ADR-028). Payload bytes `fat-hi`.
/// Passed to SVCs only (EL1 copy_from_user); never LDR'd at EL0.
const PATH: &[u8] = b"/probe";

#[inline(always)]
fn payload_matches(buf: &[u8]) -> bool {
    buf.len() >= 6
        && buf[0] == b'f'
        && buf[1] == b'a'
        && buf[2] == b't'
        && buf[3] == b'-'
        && buf[4] == b'h'
        && buf[5] == b'i'
}

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: fat-hi\n");
    yield_now();

    let fd = fs_open(PATH);
    if fd == 0 {
        let _ = uart_write(b"libctos: fat-fail\n");
        return 1;
    }
    let mut buf = [0u8; 8];
    let n = fs_read(fd, &mut buf);
    let _ = fs_close(fd);
    // `/probe` is exactly `fat-hi` (6 bytes).
    if n == u64::MAX || n as usize != 6 || !payload_matches(&buf) {
        let _ = uart_write(b"libctos: fat-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: fat-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
