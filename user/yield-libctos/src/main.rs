//! Freestanding EL0 sample that exercises cooperative yield from standing EL0
//! (ADR-062). Deeper than `hello-libctos` (single yield).
//!
//! Prints `libctos: yld-hi`, then several `libctos: beat` lines across
//! `yield_now()` rounds, then `libctos: yld-ok` and exit. Not preemption.
//! Not multi-task EL0. Not a process table. CRT `_start` lives in
//! `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

/// More than hello's single yield; still cooperative and single-task.
const BEATS: u32 = 3;

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: yld-hi\n");
    let mut i = 0u32;
    while i < BEATS {
        yield_now();
        let _ = uart_write(b"libctos: beat\n");
        i += 1;
    }
    yield_now();
    let _ = uart_write(b"libctos: yld-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
