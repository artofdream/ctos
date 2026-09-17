//! Freestanding `libctos` — wrappers for the A1 EL0 SVC ABI (ADR-021 / ADR-022)
//! plus A6 memfs numbers (ADR-027), Track N net numbers (ADR-068 / ADR-071 / ADR-076),
//! and EL0 `fs_mkdir` (ADR-075).
//!
//! Public numbers: `exit` = 16, `uart_write` = 17, `yield` = 18,
//! `fs_create` = 19, `fs_open` = 20, `fs_read` = 21, `fs_write` = 22,
//! `fs_close` = 23, `net_mac` = 24, `net_ping` = 25, `net_udp_dns` = 26,
//! `fs_mkdir` = 27, `net_tcp_echo` = 28.
//! Reserved 0–2 stay ADR-013 probes. This crate must not issue them.
//!
//! Not Linux. Not POSIX. Not glibc. Not a process model. Not a TCP product stack.
//! Not BSD sockets. Not a DNS product. Not POSIX `mkdir`.

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
/// Create a FAT16 root directory via thin VFS (ADR-075). See `FS_MKDIR_*` returns.
pub const SYS_FS_MKDIR: u64 = 27;
/// Kernel-path thin TCP echo vs guestfwd 10.0.2.4:7 (ADR-076). Returns 0, or `u64::MAX`.
pub const SYS_NET_TCP_ECHO: u64 = 28;

/// `fs_mkdir` success.
pub const FS_MKDIR_OK: u64 = 0;
/// `fs_mkdir` name already exists (file or directory).
pub const FS_MKDIR_EXISTS: u64 = 1;
/// `fs_mkdir` path rejected (grammar / nested / memfs).
pub const FS_MKDIR_BAD_PATH: u64 = 2;
/// `fs_mkdir` other fail-closed reject (Missing mount, Full, …).
pub const FS_MKDIR_ERR: u64 = u64::MAX;

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
const _: () = assert!(SYS_FS_MKDIR == 27);
const _: () = assert!(SYS_NET_TCP_ECHO == 28);
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
    pub fn ctos_fs_mkdir(ptr: *const u8, len: usize) -> u64;
    pub fn ctos_net_tcp_echo() -> u64;
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

/// `SYS_FS_MKDIR`. Returns `FS_MKDIR_OK` / `FS_MKDIR_EXISTS` / `FS_MKDIR_BAD_PATH` /
/// `FS_MKDIR_ERR`. Not POSIX `mkdir`.
#[inline]
pub fn fs_mkdir(path: &[u8]) -> u64 {
    unsafe { ctos_fs_mkdir(path.as_ptr(), path.len()) }
}

/// `SYS_NET_TCP_ECHO`. Returns 0 on thin TCP echo success, `u64::MAX` if rejected.
/// Not BSD sockets. Not listen/accept.
#[inline]
pub fn net_tcp_echo() -> u64 {
    unsafe { ctos_net_tcp_echo() }
}
