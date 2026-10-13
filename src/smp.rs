//! ADR-100 S1: opt-in secondary-CPU bring-up smoke (`--features smp2-probe`).
//!
//! The primary asks QEMU `virt` PSCI (HVC conduit, no EL2/EL3 firmware) to
//! power on MPIDR Aff0=1 at `_secondary_entry` (a separate symbol in the
//! `_start` stub page; `_start` itself is untouched). The secondary runs
//! **MMU off** at EL1: it never touches page tables, the heap, the scheduler,
//! the UART mutex or any EL0 state. It writes one raw serial line with its
//! MPIDR_EL1, stores MPIDR to a flag word whose PA the primary passed in
//! `context_id`, then parks in `wfe` forever.
//!
//! This is a bring-up smoke, not SMP. The default build has none of this
//! code and QEMU runs one core (ADR-091 rev 4 scope is unchanged).

use core::arch::{asm, global_asm};
use core::fmt::Write;
use core::sync::atomic::{AtomicU64, Ordering};

/// PSCI 0.2+ CPU_ON, SMC64/HVC64 calling convention.
const PSCI_CPU_ON_64: u64 = 0xC400_0003;
/// Target core: MPIDR affinity Aff0=1 on QEMU `virt` `-smp 2`.
const TARGET_MPIDR: u64 = 1;
/// Secondary never wrote: sentinel kept distinct from any real MPIDR.
const NOT_SEEN: u64 = 0;

/// Written by the secondary (MMU off, non-cacheable store) with MPIDR_EL1.
static SECONDARY_MPIDR: AtomicU64 = AtomicU64::new(NOT_SEEN);

global_asm!(
    r#"
    .section .text.boot, "ax"
    .global _secondary_entry
    // x0 = PSCI context_id = PA of SECONDARY_MPIDR. MMU off, EL1, no stack.
    _secondary_entry:
        mrs x9, mpidr_el1
        movz x10, #0x0900, lsl #16      // PL011 DR (PA; MMU off)
        adr x11, 9f
    1:
        ldrb w12, [x11], #1
        cbz w12, 2f
        str w12, [x10]
        b 1b
    2:
        mov x13, #60
    3:
        lsr x12, x9, x13
        and x12, x12, #0xf
        cmp x12, #10
        b.lt 4f
        add x12, x12, #('a' - 10)
        b 5f
    4:
        add x12, x12, #'0'
    5:
        str w12, [x10]
        subs x13, x13, #4
        b.ge 3b
        mov w12, #'\r'
        str w12, [x10]
        mov w12, #'\n'
        str w12, [x10]
        str x9, [x0]
        dsb sy
        sev
    6:
        wfe
        b 6b
    9:
        .asciz "smp: secondary up mpidr=0x"
        .balign 4
    .text
    "#
);

extern "C" {
    fn _secondary_entry();
}

fn mpidr() -> u64 {
    let v: u64;
    unsafe { asm!("mrs {v}, mpidr_el1", v = out(reg) v, options(nomem, nostack)) };
    v
}

fn psci_cpu_on(target: u64, entry_pa: u64, ctx: u64) -> i64 {
    let mut x0 = PSCI_CPU_ON_64;
    unsafe {
        asm!("hvc #0", inout("x0") x0, inout("x1") target => _,
             inout("x2") entry_pa => _, inout("x3") ctx => _,
             options(nostack));
    }
    x0 as i64
}

fn cntfrq() -> u64 {
    let f: u64;
    unsafe { asm!("mrs {f}, cntfrq_el0", f = out(reg) f, options(nomem, nostack)) };
    f
}

fn clean_inval(va: usize) {
    unsafe { asm!("dc civac, {x}", "dsb sy", x = in(reg) va, options(nostack)) };
}

/// Prints `smp: cpu0 mpidr=…`, `smp: cpu_on target=0x1 rc=…` and either
/// `smp: ok cpu1 mpidr=…` (flag seen) or `smp: cpu1 missed`.
pub fn observe_probe() -> bool {
    let mut w = crate::uart::raw();
    let _ = writeln!(w, "smp: cpu0 mpidr={:#018x}", mpidr());
    let flag_va = core::ptr::addr_of!(SECONDARY_MPIDR) as usize;
    SECONDARY_MPIDR.store(NOT_SEEN, Ordering::SeqCst);
    clean_inval(flag_va);
    let entry_pa = crate::paging::identity_pa(_secondary_entry as *const () as usize as u64);
    let flag_pa = crate::paging::identity_pa(flag_va as u64);
    let rc = psci_cpu_on(TARGET_MPIDR, entry_pa, flag_pa);
    // Do not print while the secondary may be writing its line.
    let freq = cntfrq().max(1);
    let t0 = crate::timer::cntpct();
    let mut seen = NOT_SEEN;
    while crate::timer::cntpct().wrapping_sub(t0) < freq / 2 {
        clean_inval(flag_va);
        seen = unsafe { core::ptr::read_volatile(flag_va as *const u64) };
        if seen != NOT_SEEN {
            break;
        }
        core::hint::spin_loop();
    }
    // Let the secondary finish its CR/LF before the primary prints.
    let t1 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t1) < freq / 20 {
        core::hint::spin_loop();
    }
    let _ = writeln!(w, "smp: cpu_on target={TARGET_MPIDR:#x} rc={rc}");
    if rc == 0 && seen & 0xff_00ff_ffff == TARGET_MPIDR {
        let _ = writeln!(w, "smp: ok cpu1 mpidr={seen:#018x} parked");
        true
    } else {
        let _ = writeln!(w, "smp: cpu1 missed seen={seen:#x}");
        false
    }
}
