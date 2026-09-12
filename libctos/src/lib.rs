//! Freestanding `libctos` — wrappers for the A1 EL0 SVC ABI (ADR-021 / ADR-022).
//!
//! Public numbers: `exit` = 16, `uart_write` = 17, `yield` = 18.
//! Reserved 0–2 stay ADR-013 probes. This crate must not issue them.
//!
//! Not Linux. Not POSIX. Not glibc. Not a process model.

#![no_std]

use core::arch::global_asm;

global_asm!(include_str!("sys.S"));

/// Halt the EL0 trip. `status` is recorded by the kernel. Does not return.
pub const SYS_EXIT: u64 = 16;
/// Write up to 64 user-mapped bytes to the virt PL011.
pub const SYS_UART_WRITE: u64 = 17;
/// Cooperative hint. Dispatch-only; does not switch EL1 tasks.
pub const SYS_YIELD: u64 = 18;

/// ADR-013 probe immediates. CRT / libctos must not issue these.
pub const SVC_PROBE_RETURN: u64 = 0;
pub const SVC_PROBE_STANDING: u64 = 1;
pub const SVC_PROBE_RESTORE: u64 = 2;

const _: () = assert!(SYS_EXIT == 16 && SYS_UART_WRITE == 17 && SYS_YIELD == 18);
const _: () = assert!(SYS_EXIT != SVC_PROBE_RETURN);
const _: () = assert!(SYS_EXIT != SVC_PROBE_STANDING);
const _: () = assert!(SYS_EXIT != SVC_PROBE_RESTORE);

extern "C" {
    pub fn ctos_exit(status: u64) -> !;
    pub fn ctos_uart_write(ptr: *const u8, len: usize) -> usize;
    pub fn ctos_yield();
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
