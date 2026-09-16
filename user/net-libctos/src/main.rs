//! Freestanding EL0 sample that exercises net SVCs (ADR-068 / Track N N4).
//!
//! Reads the guest MAC via `net_mac`, then asks the kernel to ICMP-echo the
//! SLIRP gateway via `net_ping`. Kernel still owns virtio-net — no guest
//! driver in EL0. Not TCP/UDP. Not sockets. Not DHCP/DNS. Not "has networking."
//! CRT `_start` lives in `libctos/src/crt0.S`.

#![no_std]
#![no_main]

use core::arch::global_asm;
use libctos::{exit, net_mac, net_ping, uart_write, yield_now};

global_asm!(include_str!("../../../libctos/src/crt0.S"));

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let _ = uart_write(b"libctos: net-hi\n");
    yield_now();

    let mut mac = [0u8; 6];
    let n = net_mac(&mut mac);
    // Non-zero MAC proof without formatting (EL0 exec pages are fetch-only for
    // string loads to SVCs; comparing bytes is fine on the stack buffer).
    if n != 6 || !mac.iter().any(|&b| b != 0) {
        let _ = uart_write(b"libctos: net-fail\n");
        return 1;
    }
    let _ = uart_write(b"libctos: net-mac\n");

    if net_ping() != 0 {
        let _ = uart_write(b"libctos: net-fail\n");
        return 1;
    }

    let _ = uart_write(b"libctos: net-ok\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1);
}
