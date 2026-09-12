//! Hello payload linked against `libctos` (Track A / A2 / ADR-022).
//!
//! Prints via `uart_write`, calls `yield`, then `exit`. Not a hosted app.
//! CRT `_start` lives in `libctos/src/crt0.S` (linked by the user linker script).

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: hi\n");
    yield_now();
    let _ = uart_write(b"libctos: ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
