//! Freestanding EL0 sample that exercises libctos VFS wrappers (ADR-059).
//!
//! Creates/opens/writes/reads/closes a memfs path under the ADR-058 `/mem`
//! prefix (`/memdemo`). Flat path grammar forbids an extra `/` (`/mem/...`).
//!
//! Important: EL0 exec pages are fetch-only (AP no EL0 data; see
//! `paging::l3_page_el0_exec`). String literals may be passed to SVCs (EL1
//! copies them) but must not be loaded from EL0. Compare with immediates.
//! Not POSIX. Not getdents. CRT `_start` lives in `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, fs_close, fs_create, fs_open, fs_read, fs_write, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

/// Hunch (verified by ADR-058 mount table): `/memdemo` longest-prefix-matches
/// `/mem` → memfs. Not a FAT path; the ELF itself lives on FAT `/fsdemo`.
/// Passed to SVCs only (EL1 copy_from_user); never LDR'd at EL0.
const PATH: &[u8] = b"/memdemo";
/// Written via SVC; bytes re-checked with immediates at EL0 (no .rodata LDR).
const PAYLOAD: &[u8] = b"fs-hi";

#[inline(always)]
fn payload_matches(buf: &[u8]) -> bool {
    buf.len() >= 5
        && buf[0] == b'f'
        && buf[1] == b's'
        && buf[2] == b'-'
        && buf[3] == b'h'
        && buf[4] == b'i'
}

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: fs-hi\n");
    yield_now();

    let mut fd = fs_create(PATH);
    if fd == 0 {
        // Re-entry: path may already exist on memfs.
        fd = fs_open(PATH);
    }
    if fd == 0 {
        let _ = uart_write(b"libctos: fs-fail\n");
        return 1;
    }
    let n = fs_write(fd, PAYLOAD);
    if n != PAYLOAD.len() as u64 {
        let _ = fs_close(fd);
        let _ = uart_write(b"libctos: fs-fail\n");
        return 1;
    }
    if fs_close(fd) != 0 {
        let _ = uart_write(b"libctos: fs-fail\n");
        return 1;
    }

    let fd = fs_open(PATH);
    if fd == 0 {
        let _ = uart_write(b"libctos: fs-fail\n");
        return 1;
    }
    let mut buf = [0u8; 8];
    let n = fs_read(fd, &mut buf);
    let _ = fs_close(fd);
    if n == u64::MAX || n as usize != PAYLOAD.len() || !payload_matches(&buf) {
        let _ = uart_write(b"libctos: fs-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: fs-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
