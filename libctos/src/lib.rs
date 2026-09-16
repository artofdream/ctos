//! Freestanding `libctos` — wrappers for the A1 EL0 SVC ABI (ADR-021 / ADR-022)
//! plus A6 memfs numbers (ADR-027) and Track N net numbers (ADR-068 / ADR-071).
//!
//! Public numbers: `exit` = 16, `uart_write` = 17, `yield` = 18,
//! `fs_create` = 19, `fs_open` = 20, `fs_read` = 21, `fs_write` = 22,
//! `fs_close` = 23, `net_mac` = 24, `net_ping` = 25, `net_udp_dns` = 26.
//! Reserved 0–2 stay ADR-013 probes. This crate must not issue them.
//!
//! Not Linux. Not POSIX. Not glibc. Not a process model. Not FAT.
//! Not a TCP stack. Not BSD sockets. Not a DNS product.

#![no_std]

use core::arch::global_asm;

global_asm!(include_str!("sys.S"));

/// Halt the EL0 trip. `status` is recorded by the kernel. Does not return.
pub const SYS_EXIT: u64 = 16;
/// Write up to 64 user-mapped bytes to the virt PL011.
pub const SYS_UART_WRITE: u64 = 17;
/// Cooperative hint. Dispatch-only; does not switch EL1 tasks.
pub const SYS_YIELD: u64 = 18;
/// Create an empty memfs path. Returns fd `>= 1`, or `0`.
pub const SYS_FS_CREATE: u64 = 19;
/// Open an existing memfs path. Returns fd `>= 1`, or `0`.
pub const SYS_FS_OPEN: u64 = 20;
/// Read from a memfs handle. Returns byte count, or `u64::MAX` on reject.
pub const SYS_FS_READ: u64 = 21;
/// Write to a memfs handle. Returns byte count, or `0`.
pub const SYS_FS_WRITE: u64 = 22;
/// Close a memfs handle. Returns `0` on success, `u64::MAX` on reject.
pub const SYS_FS_CLOSE: u64 = 23;
/// Copy the guest virtio-net MAC into a user buffer (6 bytes). Returns 6, or 0.
pub const SYS_NET_MAC: u64 = 24;
/// Kernel-path ICMP echo to SLIRP gateway. Returns 0 on success, `u64::MAX` on fail.
pub const SYS_NET_PING: u64 = 25;
/// Kernel-path UDP DNS probe to SLIRP 10.0.2.3:53 (ADR-071). Returns 0, or `u64::MAX`.
pub const SYS_NET_UDP_DNS: u64 = 26;

/// ADR-013 probe immediates. CRT / libctos must not issue these.
pub const SVC_PROBE_RETURN: u64 = 0;
pub const SVC_PROBE_STANDING: u64 = 1;
pub const SVC_PROBE_RESTORE: u64 = 2;

const _: () = assert!(SYS_EXIT == 16 && SYS_UART_WRITE == 17 && SYS_YIELD == 18);
const _: () = assert!(
    SYS_FS_CREATE == 19
        && SYS_FS_OPEN == 20
        && SYS_FS_READ == 21
        && SYS_FS_WRITE == 22
        && SYS_FS_CLOSE == 23
);
const _: () = assert!(SYS_NET_MAC == 24 && SYS_NET_PING == 25 && SYS_NET_UDP_DNS == 26);
const _: () = assert!(SYS_EXIT != SVC_PROBE_RETURN);
const _: () = assert!(SYS_EXIT != SVC_PROBE_STANDING);
const _: () = assert!(SYS_EXIT != SVC_PROBE_RESTORE);

extern "C" {
    pub fn ctos_exit(status: u64) -> !;
    pub fn ctos_uart_write(ptr: *const u8, len: usize) -> usize;
    pub fn ctos_yield();
    pub fn ctos_fs_create(ptr: *const u8, len: usize) -> u64;
    pub fn ctos_fs_open(ptr: *const u8, len: usize) -> u64;
    pub fn ctos_fs_read(fd: u64, ptr: *mut u8, len: usize) -> u64;
    pub fn ctos_fs_write(fd: u64, ptr: *const u8, len: usize) -> u64;
    pub fn ctos_fs_close(fd: u64) -> u64;
    pub fn ctos_net_mac(ptr: *mut u8, len: usize) -> u64;
    pub fn ctos_net_ping() -> u64;
    pub fn ctos_net_udp_dns() -> u64;
}

/// End the EL0 trip. Does not return to the caller.
#[inline]
pub fn exit(status: u64) -> ! {
    unsafe { ctos_exit(status) }
}

/// Write `buf` via `SYS_UART_WRITE`. Returns the kernel count (0 if rejected).
#[inline]
pub fn uart_write(buf: &[u8]) -> usize {
    unsafe { ctos_uart_write(buf.as_ptr(), buf.len()) }
}

/// `SYS_YIELD`. Returns after the kernel `ERET`s back to EL0.
#[inline]
pub fn yield_now() {
    unsafe { ctos_yield() }
}

/// `SYS_FS_CREATE`. Returns fd `>= 1`, or `0`.
#[inline]
pub fn fs_create(path: &[u8]) -> u64 {
    unsafe { ctos_fs_create(path.as_ptr(), path.len()) }
}

/// `SYS_FS_OPEN`. Returns fd `>= 1`, or `0`.
#[inline]
pub fn fs_open(path: &[u8]) -> u64 {
    unsafe { ctos_fs_open(path.as_ptr(), path.len()) }
}

/// `SYS_FS_READ`. Returns bytes copied, or `u64::MAX` if rejected.
#[inline]
pub fn fs_read(fd: u64, buf: &mut [u8]) -> u64 {
    unsafe { ctos_fs_read(fd, buf.as_mut_ptr(), buf.len()) }
}

/// `SYS_FS_WRITE`. Returns bytes stored, or `0` if rejected.
#[inline]
pub fn fs_write(fd: u64, buf: &[u8]) -> u64 {
    unsafe { ctos_fs_write(fd, buf.as_ptr(), buf.len()) }
}

/// `SYS_FS_CLOSE`. Returns `0` on success, `u64::MAX` if rejected.
#[inline]
pub fn fs_close(fd: u64) -> u64 {
    unsafe { ctos_fs_close(fd) }
}

/// `SYS_NET_MAC`. Copies 6 MAC bytes into `buf`. Returns 6, or 0 if rejected.
#[inline]
pub fn net_mac(buf: &mut [u8]) -> u64 {
    unsafe { ctos_net_mac(buf.as_mut_ptr(), buf.len()) }
}

/// `SYS_NET_PING`. Returns 0 on ICMP echo success, `u64::MAX` if rejected.
#[inline]
pub fn net_ping() -> u64 {
    unsafe { ctos_net_ping() }
}

/// `SYS_NET_UDP_DNS`. Returns 0 on UDP DNS probe success, `u64::MAX` if rejected.
#[inline]
pub fn net_udp_dns() -> u64 {
    unsafe { ctos_net_udp_dns() }
}
