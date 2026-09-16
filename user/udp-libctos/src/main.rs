//! Freestanding EL0 sample that exercises the UDP DNS SVC (ADR-071 / Track N N3.x).
//!
//! Asks the kernel to send one UDP DNS query to SLIRP 10.0.2.3:53 via
//! `net_udp_dns`. Kernel still owns virtio-net — no guest driver in EL0.
//! Not BSD sockets. Not TCP. Not a DNS product. Not "has networking."
//! CRT `_start` lives in `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, net_udp_dns, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: udp-hi\n");
    yield_now();

    if net_udp_dns() != 0 {
        let _ = uart_write(b"libctos: udp-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: udp-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
