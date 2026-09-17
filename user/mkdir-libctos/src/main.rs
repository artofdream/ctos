//! Freestanding EL0 sample that exercises `fs_mkdir` (ADR-075).
//!
//! Creates FAT16 root directory `/edir` via SVC 27, proves Exists on a
//! second create, and proves BadPath on a path deeper than one nest level
//! (`/a/b/c`). One-level nested mkdir is a kernel/VFS mile (ADR-077), not
//! this sample. Not POSIX `mkdir`.
//! CRT `_start` lives in `libctos/src/crt0.S`.
//!
//! Important: EL0 exec pages are fetch-only. Path bytes are passed to SVCs
//! (EL1 copy) and must not be `LDR`'d at EL0.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{
    exit, fs_mkdir, uart_write, yield_now, FS_MKDIR_BAD_PATH, FS_MKDIR_EXISTS, FS_MKDIR_OK,
};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

/// EL0 mkdir target (separate from kernel `/fdir`).
const DIR_PATH: &[u8] = b"/edir";

/// Three-level path — mkdir/rmdir grammar allows at most one nest (ADR-077).
const BAD_PATH: &[u8] = b"/a/b/c";

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: mkdir-hi\n");
    yield_now();

    // First create: Ok (0) or Exists (1) if a prior trip already made `/edir`.
    let first = fs_mkdir(DIR_PATH);
    if first != FS_MKDIR_OK && first != FS_MKDIR_EXISTS {
        let _ = uart_write(b"libctos: mkdir-fail\n");
        return 1;
    }

    // Second create must be Exists.
    if fs_mkdir(DIR_PATH) != FS_MKDIR_EXISTS {
        let _ = uart_write(b"libctos: mkdir-fail\n");
        return 1;
    }

    // Deeper than one nest level must be BadPath.
    if fs_mkdir(BAD_PATH) != FS_MKDIR_BAD_PATH {
        let _ = uart_write(b"libctos: mkdir-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: mkdir-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
