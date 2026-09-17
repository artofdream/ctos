//! Freestanding EL0 sample that exercises the TCP echo SVC (ADR-076).
//!
//! Asks the kernel to run the same quiet thin-TCP guestfwd echo probe as
//! ADR-074 via `net_tcp_echo`. Kernel still owns virtio-net — no guest
//! driver in EL0. Not BSD sockets. Not listen/accept. Not TLS. Not a
//! multi-connection table. Not "has networking."
//! CRT `_start` lives in `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, net_tcp_echo, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: tcp-hi\n");
    yield_now();

    if net_tcp_echo() != 0 {
        let _ = uart_write(b"libctos: tcp-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: tcp-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
