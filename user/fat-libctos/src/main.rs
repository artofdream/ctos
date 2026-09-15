//! Freestanding EL0 sample that exercises FAT via thin VFS (ADR-061 + ADR-064).
//!
//! 1. Opens/reads the existing FAT16 `/probe` payload (`fat-hi`) through
//!    libctos `fs_open`/`fs_read`/`fs_close` (SVC 20/21/23).
//! 2. Creates `/egrow`, writes past one cluster via chunked `fs_write`
//!    (SVC 22, `FS_IO_MAX` = 64), then read-back proves the cluster-boundary
//!    bytes (ADR-064 multi-cluster grow).
//!
//! Thin VFS routes `/` → FAT16 (ADR-058). Deeper than `fs-libctos` (`/memdemo`
//! memfs only).
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

/// Existing FAT16 probe file (A7 / ADR-028). Payload bytes `fat-hi`.
/// Passed to SVCs only (EL1 copy_from_user); never LDR'd at EL0.
const PATH: &[u8] = b"/probe";

/// EL0 multi-cluster grow target (ADR-064). Separate from kernel `/fgrow`.
const GROW_PATH: &[u8] = b"/egrow";

/// Must exceed one FAT cluster (512 bytes with mkfat16 spc=1).
const GROW_TOTAL: usize = 600;
const CHUNK: usize = 64;

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

#[inline(always)]
fn grow_byte(i: usize) -> u8 {
    b'A'.wrapping_add((i % 26) as u8)
}

/// Create-or-open `/egrow`, write `GROW_TOTAL` bytes in `CHUNK` SVC writes,
/// then read back boundary bytes across the cluster edge.
#[inline(never)]
fn prove_multi_cluster_grow() -> bool {
    let mut fd = fs_create(GROW_PATH);
    if fd == 0 {
        fd = fs_open(GROW_PATH);
    }
    if fd == 0 {
        return false;
    }

    let mut off = 0usize;
    while off < GROW_TOTAL {
        let n = core::cmp::min(CHUNK, GROW_TOTAL - off);
        let mut chunk = [0u8; CHUNK];
        for i in 0..n {
            chunk[i] = grow_byte(off + i);
        }
        let w = fs_write(fd, &chunk[..n]);
        if w == 0 || w as usize != n {
            let _ = fs_close(fd);
            return false;
        }
        off += n;
    }
    if fs_close(fd) == u64::MAX {
        return false;
    }

    let fd = fs_open(GROW_PATH);
    if fd == 0 {
        return false;
    }

    // Walk the file in CHUNK reads; check first byte, last-of-cluster, first-of-next.
    let mut got = 0usize;
    let mut edge_lo = 0u8;
    let mut edge_hi = 0u8;
    let mut first = 0u8;
    while got < GROW_TOTAL {
        let mut buf = [0u8; CHUNK];
        let want = core::cmp::min(CHUNK, GROW_TOTAL - got);
        let n = fs_read(fd, &mut buf[..want]);
        if n == u64::MAX || n as usize != want {
            let _ = fs_close(fd);
            return false;
        }
        for i in 0..want {
            let idx = got + i;
            if buf[i] != grow_byte(idx) {
                let _ = fs_close(fd);
                return false;
            }
            if idx == 0 {
                first = buf[i];
            }
            if idx == 511 {
                edge_lo = buf[i];
            }
            if idx == 512 {
                edge_hi = buf[i];
            }
        }
        got += want;
    }
    let _ = fs_close(fd);

    // Immediates: pattern A..Z → off0='A', 511 and 512 differ.
    first == b'A' && edge_lo != edge_hi && edge_lo == grow_byte(511) && edge_hi == grow_byte(512)
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

    if !prove_multi_cluster_grow() {
        let _ = uart_write(b"libctos: fat-fail\n");
        return 1;
    }
    let _ = uart_write(b"libctos: fat-grow\n");

    let _ = uart_write(b"libctos: fat-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
