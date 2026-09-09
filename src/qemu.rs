//! Fail-closed QEMU exit for `virt` via ARM semihosting.
//!
//! This is the AArch64 stand-in for x86 `isa-debug-exit`. QEMU must be
//! started with `-semihosting` (or `-semihosting-config enable=on`).
//! Without that flag, `hlt #0xf000` does not stop the VM.
//!
//! A64 SYS_EXIT (0x18) takes a parameter block: reason
//! `ADP_Stopped_ApplicationExit` (0x20026) and a host exit status.
//! See ARM "Semihosting for AArch32 and AArch64" and QEMU
//! `semihosting/arm-compat-semi.c`.

#![allow(dead_code)] // used from `cfg(test)` / `force-fail`; kept in the default crate.

const SYS_EXIT: u64 = 0x18;
const ADP_STOPPED_APPLICATION_EXIT: u64 = 0x20026;

/// Host process exit status for a passing test suite.
pub const SUCCESS: u64 = 0;
/// Host process exit status for a failing test / panic.
pub const FAILURE: u64 = 1;

pub fn exit_success() -> ! {
    exit(SUCCESS)
}

pub fn exit_failure() -> ! {
    exit(FAILURE)
}

pub fn exit(code: u64) -> ! {
    let block = [ADP_STOPPED_APPLICATION_EXIT, code];
    unsafe {
        // Do not mark this `nomem`: QEMU reads the parameter block.
        core::arch::asm!(
            "hlt #0xf000",
            in("x0") SYS_EXIT,
            in("x1") block.as_ptr(),
        );
    }
    loop {
        unsafe {
            core::arch::asm!("wfe", options(nomem, nostack));
        }
    }
}
