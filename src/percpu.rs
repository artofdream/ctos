//! ADR-104 (S4): per-CPU state reached through `TPIDR_EL1`.
//!
//! `TPIDR_EL1` holds the TTBR1 high VA of this core's [`PerCpu`] block (0
//! until the core installs it; [`id`] then reads as core 0). Offset 0 is the
//! saved kernel TTBR0 that the lower-EL exception trampolines restore before
//! touching `.data` (previously `TPIDR_EL1` held that value directly).
//!
//! [`Banked`] gives each core its own copy of a static: the EL0 context and
//! the expected-fault / caught flags in `el0.rs` and `exception.rs` deref to
//! the copy of the core that reads them. Code that uses them is unchanged.
//! On one core only slot 0 is ever used.

use core::sync::atomic::{AtomicU64, Ordering};

/// Cores with a block (QEMU `virt` `-smp 2` at most for the SMP probes).
pub const NCPU: usize = 2;

/// Layout is used by the trampolines in `exception.rs` (offset 0).
#[repr(C, align(64))]
pub struct PerCpu {
    /// Saved kernel TTBR0 (offset 0; read by assembly).
    pub kernel_ttbr0: AtomicU64,
    /// Core index (0 = boot core).
    pub id: AtomicU64,
    /// Last ASID generation this core has observed (ADR-104 allocator).
    pub asid_gen_seen: AtomicU64,
    /// Top of this core's fatal / nested-exception stack (offset 24; read by
    /// `fatal_enter`). 0 = the linker `__fatal_stack_top` (core 0). ADR-105.
    pub fatal_top: AtomicU64,
    /// Task running on this core: run-queue id + 1, 0 = none (ADR-105).
    pub current: AtomicU64,
}

/// All-zero (`.bss`). Core 0 installs its block in `kernel_main_high`
/// only after the identity tears: the ADR-037 pointer rewrite covers
/// `[__data_start, __kernel_end)` and would turn the saved TTBR0 (the kernel
/// L1 PA, itself in `.bss`) into a high VA.
static mut BLOCKS: [PerCpu; NCPU] = [
    PerCpu::ZERO,
    PerCpu::ZERO,
];

impl PerCpu {
    const ZERO: PerCpu = PerCpu {
        kernel_ttbr0: AtomicU64::new(0),
        id: AtomicU64::new(0),
        asid_gen_seen: AtomicU64::new(0),
        fatal_top: AtomicU64::new(0),
        current: AtomicU64::new(0),
    };
}

fn tpidr() -> u64 {
    let v: u64;
    unsafe { core::arch::asm!("mrs {v}, tpidr_el1", v = out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// This core's block, or `None` before [`install`] ran on it.
pub fn this() -> Option<&'static PerCpu> {
    let p = tpidr();
    if p == 0 {
        None
    } else {
        Some(unsafe { &*(p as *const PerCpu) })
    }
}

/// This core's index (0 before [`install`]).
#[inline]
pub fn id() -> usize {
    match this() {
        Some(b) => (b.id.load(Ordering::Relaxed) as usize).min(NCPU - 1),
        None => 0,
    }
}

/// Set `cpu`'s fatal-stack top (high VA) before [`install`] (ADR-105).
#[allow(dead_code)]
pub fn set_fatal_top(cpu: usize, top: u64) {
    let b = unsafe { &*core::ptr::addr_of!(BLOCKS[cpu]) };
    b.fatal_top.store(top, Ordering::SeqCst);
}

/// Fill `cpu`'s block and point `TPIDR_EL1` at its TTBR1 alias. MMU must be
/// on (the block is reached through TTBR1 from every EL0 trampoline).
pub fn install(cpu: usize, kernel_ttbr0: u64) -> u64 {
    let b = unsafe { &*core::ptr::addr_of!(BLOCKS[cpu]) };
    b.kernel_ttbr0.store(kernel_ttbr0, Ordering::SeqCst);
    b.id.store(cpu as u64, Ordering::SeqCst);
    let va = crate::paging::to_high_va(b as *const PerCpu as u64);
    unsafe {
        core::arch::asm!("dsb ish", "msr tpidr_el1, {v}", "isb", v = in(reg) va, options(nostack, preserves_flags));
    }
    va
}

/// One copy of `T` per core; derefs to the reading core's copy.
pub struct Banked<T>(pub [T; NCPU]);

impl<T> core::ops::Deref for Banked<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0[id()]
    }
}

/// `banked!(AtomicBool::new(false))` → one fresh value per core.
#[macro_export]
macro_rules! banked {
    ($e:expr) => {
        $crate::percpu::Banked([$e, $e])
    };
}
