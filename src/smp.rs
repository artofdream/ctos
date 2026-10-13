//! ADR-100 S1 + ADR-102 S2: opt-in secondary-CPU probe (`--features smp2-probe`).
//!
//! S1 (kept): the primary asks QEMU `virt` PSCI (HVC conduit, no EL2/EL3
//! firmware) to power on MPIDR Aff0=1 at `_secondary_entry` (a separate
//! symbol in the `_start` stub page; `_start` itself is untouched). MMU off,
//! the secondary writes one raw serial line with its MPIDR and stores MPIDR
//! to the first word of a parameter block whose PA the primary passed in
//! `context_id`.
//!
//! S2 (ADR-102): instead of parking MMU off, the secondary loads MAIR / TCR /
//! TTBR0 / TTBR1 / SCTLR from that block (the values core 0 is running with,
//! except SCTLR.SPAN forced back to its reset value 1), turns the MMU on
//! (shared kernel tables), switches to its own stack, and branches to
//! [`cpu1_main`] at the TTBR1 high alias. There it installs **its own**
//! vector table, clears SCTLR.SPAN and sets PSTATE.PAN itself, reads all of
//! it back, then takes one deliberate `brk` that its own vectors handle. All
//! post-MMU output goes through the `UART` spin lock, one line per lock hold.
//!
//! Not SMP. The secondary never runs EL0, the scheduler, the heap, page-table
//! builders, drivers or interrupts. The default build has none of this code
//! and QEMU runs one core (ADR-091 rev 4 scope is unchanged).

use core::arch::{asm, global_asm};
use core::fmt::Write;
use core::sync::atomic::{AtomicU64, Ordering};

/// PSCI 0.2+ CPU_ON, SMC64/HVC64 calling convention.
const PSCI_CPU_ON_64: u64 = 0xC400_0003;
/// Target core: MPIDR affinity Aff0=1 on QEMU `virt` `-smp 2`.
const TARGET_MPIDR: u64 = 1;
/// Secondary never wrote: sentinel kept distinct from any real MPIDR.
const NOT_SEEN: u64 = 0;
/// `brk` immediate the secondary takes on purpose (ADR-102).
const BRK_IMM: u64 = 0x5102;
/// ESR_EL1 for `brk #BRK_IMM` from AArch64: EC=0x3C, IL=1, ISS=imm16.
const BRK_ESR: u64 = (0x3c << 26) | (1 << 25) | BRK_IMM;
const SCTLR_M: u64 = 1 << 0;
const SCTLR_SPAN: u64 = 1 << 23;
/// Console-lock lines each core prints while the other is live.
const TICKS: u64 = 4;

/// Verdict codes the secondary publishes in `Block::verdict`.
const V_NONE: u64 = 0;
const V_OK: u64 = 1;
const V_MMU: u64 = 2;
const V_PAN: u64 = 3;
const V_SPAN: u64 = 4;
const V_VBAR: u64 = 5;
const V_EXC: u64 = 6;
const V_PAN_HANDLER: u64 = 7;

/// Parameter block. Offsets are used by `_secondary_entry` (MMU off).
#[repr(C, align(64))]
struct Block {
    mpidr: AtomicU64, // 0: written MMU off by the secondary
    mair: u64,        // 8
    tcr: u64,         // 16
    ttbr0: u64,       // 24
    ttbr1: u64,       // 32
    sctlr: u64,       // 40
    sp: u64,          // 48: high VA, top of SECONDARY_STACK
    entry: u64,       // 56: high VA of cpu1_main
    arg: u64,         // 64: high VA of this block
    verdict: AtomicU64,
    ticks: AtomicU64,
    go: AtomicU64,
    xtlb_va: AtomicU64,
    xtlb_phase: AtomicU64,
    xtlb_result: AtomicU64,
}

static mut BLOCK: Block = Block {
    mpidr: AtomicU64::new(NOT_SEEN),
    mair: 0,
    tcr: 0,
    ttbr0: 0,
    ttbr1: 0,
    sctlr: 0,
    sp: 0,
    entry: 0,
    arg: 0,
    verdict: AtomicU64::new(V_NONE),
    ticks: AtomicU64::new(0),
    go: AtomicU64::new(0),
    xtlb_va: AtomicU64::new(0),
    xtlb_phase: AtomicU64::new(0),
    xtlb_result: AtomicU64::new(0),
};

/// Secondary's own stack (no guard page; ADR-102 limits).
#[repr(C, align(4096))]
struct Stack([u8; 16 * 1024]);
static mut SECONDARY_STACK: Stack = Stack([0; 16 * 1024]);

/// Written by the secondary's own sync handler: ESR, ELR, PSTATE.PAN at
/// entry, entry count, and a marker for any vector slot other than sync.
#[no_mangle]
static CTOS_CPU1_EXC: [AtomicU64; 6] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(u64::MAX),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

global_asm!(
    r#"
    .section .text.boot, "ax"
    .global _secondary_entry
    // x0 = PSCI context_id = PA of BLOCK. MMU off, EL1, SPSel=1, no stack.
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
        // ADR-102 S2: core 0's translation regime, then MMU on.
        ldr x1, [x0, #8]
        msr mair_el1, x1
        ldr x1, [x0, #16]
        msr tcr_el1, x1
        ldr x1, [x0, #24]
        msr ttbr0_el1, x1
        ldr x1, [x0, #32]
        msr ttbr1_el1, x1
        isb
        tlbi vmalle1
        dsb nsh
        isb
        ldr x2, [x0, #48]
        ldr x3, [x0, #56]
        ldr x4, [x0, #64]
        ldr x1, [x0, #40]
        msr sctlr_el1, x1
        isb
        mov sp, x2
        mov x0, x4
        br x3
    9:
        .asciz "smp: secondary up mpidr=0x"
        .balign 4
    .text

    // ADR-102: the secondary's own vector table (not core 0's). Only the
    // current-EL SPx sync slot is expected; every other slot records which
    // slot fired and parks the core (core 0 then reports a miss).
    .balign 2048
    .global ctos_cpu1_vectors
    ctos_cpu1_vectors:
    .irp slot, 0,1,2,3
        .balign 128
        mov x0, #\slot
        b ctos_cpu1_bad
    .endr
        .balign 128
        b ctos_cpu1_sync
    .irp slot, 5,6,7,8,9,10,11,12,13,14,15
        .balign 128
        mov x0, #\slot
        b ctos_cpu1_bad
    .endr
    ctos_cpu1_sync:
        stp x0, x1, [sp, #-16]!
        stp x2, x3, [sp, #-16]!
        adrp x0, CTOS_CPU1_EXC
        add x0, x0, :lo12:CTOS_CPU1_EXC
        mrs x1, esr_el1
        str x1, [x0]
        mrs x2, elr_el1
        str x2, [x0, #8]
        mrs x1, far_el1
        str x1, [x0, #40]
        mrs x1, pan
        lsr x1, x1, #22
        str x1, [x0, #16]
        ldr x1, [x0, #24]
        add x1, x1, #1
        str x1, [x0, #24]
        add x2, x2, #4
        msr elr_el1, x2
        dmb ish
        ldp x2, x3, [sp], #16
        ldp x0, x1, [sp], #16
        eret
    ctos_cpu1_bad:
        adrp x1, CTOS_CPU1_EXC
        add x1, x1, :lo12:CTOS_CPU1_EXC
        add x0, x0, #0x100
        str x0, [x1, #32]
        dmb ish
    10:
        wfe
        b 10b
    "#
);

extern "C" {
    fn _secondary_entry();
    static ctos_cpu1_vectors: u8;
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

macro_rules! sysreg {
    ($name:literal) => {{
        let v: u64;
        unsafe { asm!(concat!("mrs {v}, ", $name), v = out(reg) v, options(nomem, nostack)) };
        v
    }};
}

/// High VA of the secondary's own vector table.
fn cpu1_vbar() -> u64 {
    crate::paging::to_high_va(core::ptr::addr_of!(ctos_cpu1_vectors) as u64)
}

/// One console line under the `UART` lock (ADR-102 console lock).
fn line(args: core::fmt::Arguments) {
    let mut g = crate::uart::UART.lock();
    let _ = g.write_fmt(args);
    let _ = g.write_str("\n");
}

fn pstate_pan() -> u64 {
    (sysreg!("pan") >> 22) & 1
}

/// Secondary, MMU on, own stack, TTBR1 high alias. Never returns.
extern "C" fn cpu1_main(block: &'static Block) -> ! {
    let verdict = cpu1_checks();
    // Wait for core 0 so both cores print ticks at the same time.
    while block.go.load(Ordering::SeqCst) == 0 {
        core::hint::spin_loop();
    }
    for i in 0..TICKS {
        line(format_args!("smp: tick cpu1 {i}"));
        block.ticks.fetch_add(1, Ordering::SeqCst);
    }
    block.verdict.store(verdict, Ordering::SeqCst);
    unsafe { asm!("dsb ish", "sev", options(nostack)) };
    if verdict == V_OK {
        cpu1_xtlb(block);
    }
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)) };
    }
}

fn cpu1_checks() -> u64 {
    let own_vbar = cpu1_vbar();
    #[cfg(not(feature = "smp2-neg-vbar"))]
    let want = own_vbar;
    // Negative: install core 0's table instead of our own.
    #[cfg(feature = "smp2-neg-vbar")]
    let want = crate::paging::to_high_va(crate::exception::vector_table_addr());
    unsafe { asm!("msr vbar_el1, {v}", "isb", v = in(reg) want, options(nostack)) };
    #[cfg(not(feature = "smp2-neg-span"))]
    {
        let v = sysreg!("sctlr_el1") & !SCTLR_SPAN;
        unsafe { asm!("msr sctlr_el1, {v}", "isb", v = in(reg) v, options(nostack)) };
    }
    #[cfg(not(feature = "smp2-neg-pan"))]
    unsafe {
        asm!("msr pan, #1", "isb", options(nostack));
    }
    let sctlr = sysreg!("sctlr_el1");
    let vbar = sysreg!("vbar_el1");
    let pan = pstate_pan();
    let span = (sctlr & SCTLR_SPAN) >> 23;
    let mmu = sctlr & SCTLR_M;
    let sp: u64;
    unsafe { asm!("mov {s}, sp", s = out(reg) sp, options(nomem, nostack)) };
    let stack_lo = core::ptr::addr_of!(SECONDARY_STACK) as u64;
    let own_sp = sp > stack_lo && sp <= stack_lo + 16 * 1024 && crate::paging::is_high_va(sp);
    let pc_high = crate::paging::pc_is_high();
    line(format_args!(
        "smp: cpu1 regs mmu={mmu} pan={pan} span={span} vbar={} sp={} pc={} mpidr={:#018x}",
        if vbar == own_vbar { "own" } else { "other" },
        if own_sp { "own" } else { "other" },
        if pc_high { "high" } else { "low" },
        mpidr()
    ));
    if mmu != 1 || !pc_high || !own_sp {
        line(format_args!("smp: cpu1 mmu missed"));
        return V_MMU;
    }
    if vbar != own_vbar {
        // Do not take an exception into core 0's vectors / stacks.
        line(format_args!("smp: cpu1 vbar mismatch"));
        return V_VBAR;
    }
    if pan != 1 {
        line(format_args!("smp: cpu1 pan lost (pstate pan=0)"));
        return V_PAN;
    }
    // Deliberate exception. Clear PAN first: with SPAN=0 the handler must
    // see PAN=1 (set on entry); ERET restores PAN=0, then we set it again.
    let brk_pc: u64;
    unsafe {
        asm!(
            "msr pan, #0",
            "adr {pc}, 1f",
            "1: brk #0x5102",
            "msr pan, #1",
            "isb",
            pc = out(reg) brk_pc,
            options(nostack),
        );
    }
    let esr = CTOS_CPU1_EXC[0].load(Ordering::SeqCst);
    let elr = CTOS_CPU1_EXC[1].load(Ordering::SeqCst);
    let hpan = CTOS_CPU1_EXC[2].load(Ordering::SeqCst);
    let n = CTOS_CPU1_EXC[3].load(Ordering::SeqCst);
    let pan_after = pstate_pan();
    if n != 1 || esr != BRK_ESR || elr != brk_pc {
        line(format_args!("smp: cpu1 exc missed n={n} esr={esr:#x}"));
        return V_EXC;
    }
    if hpan != 1 {
        line(format_args!("smp: cpu1 pan lost (handler pan={hpan} span={span})"));
        return if span != 0 { V_SPAN } else { V_PAN_HANDLER };
    }
    if span != 0 || pan_after != 1 {
        line(format_args!("smp: cpu1 pan lost (span={span} after={pan_after})"));
        return V_SPAN;
    }
    line(format_args!("smp: cpu1 exc brk handled esr={esr:#x} elr=brk handler-pan=1 vbar=own"));
    V_OK
}

// ---- ADR-103 (S3): cross-core TLB probe --------------------------------
//
// Core 0 maps a page in the shared kernel TTBR0 map window; core 1 reads it
// (its TLB now caches the translation); core 0 unmaps it with the
// Inner-Shareable TLBI (`paging::unmap_page`); core 1 reads again and must
// take a translation fault on its own vectors. The `local-tlbi-leak-probe`
// build keeps local-only TLBI, so core 1 may still read the old value.

const XTLB_MAGIC: u64 = 0x5854_4c42_5f53_3301; // "XTLB_S3" + 1
/// Phases (core 0 writes 1 and 3; core 1 writes 2 and 4).
const X_MAPPED: u64 = 1;
const X_READ1: u64 = 2;
const X_UNMAPPED: u64 = 3;
const X_DONE: u64 = 4;
/// Results core 1 publishes.
const XR_FAULT: u64 = 1;
const XR_LEAK: u64 = 2;
const XR_FIRST_READ_BAD: u64 = 3;
const XR_OTHER: u64 = 4;
/// Load value before the second read; a skipped faulting load leaves it.
const XTLB_SENTINEL: u64 = 0xdead_0000_0000_0003;

#[repr(C, align(4096))]
struct Page([u64; 512]);
static mut XTLB_PAGE: Page = Page([0; 512]);

fn wait_phase(block: &Block, want: u64, ticks: u64) -> bool {
    let t0 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t0) < ticks {
        if block.xtlb_phase.load(Ordering::SeqCst) == want {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn cpu1_xtlb(block: &'static Block) {
    let freq = cntfrq().max(1);
    if !wait_phase(block, X_MAPPED, freq * 2) {
        return;
    }
    let va = block.xtlb_va.load(Ordering::SeqCst);
    let first = unsafe { core::ptr::read_volatile(va as *const u64) };
    let n0 = CTOS_CPU1_EXC[3].load(Ordering::SeqCst);
    block.xtlb_phase.store(X_READ1, Ordering::SeqCst);
    if !wait_phase(block, X_UNMAPPED, freq * 2) {
        return;
    }
    // Second read: one plain `ldr`; on a fault the handler skips it.
    let mut got: u64 = XTLB_SENTINEL;
    unsafe {
        asm!("ldr {v}, [{a}]", v = inout(reg) got, a = in(reg) va, options(nostack));
    }
    let n1 = CTOS_CPU1_EXC[3].load(Ordering::SeqCst);
    let esr = CTOS_CPU1_EXC[0].load(Ordering::SeqCst);
    let far = CTOS_CPU1_EXC[5].load(Ordering::SeqCst);
    let ec = esr >> 26;
    let dfsc = esr & 0x3f;
    let res = if first != XTLB_MAGIC {
        line(format_args!("smp: xtlb cpu1 first read bad"));
        XR_FIRST_READ_BAD
    } else if n1 == n0 + 1 && ec == 0x25 && (0x04..=0x07).contains(&dfsc) && far == va && got == XTLB_SENTINEL {
        line(format_args!("smp: xtlb cpu1 fault ec=0x25 dfsc=trans-l{} far=window own-vectors", dfsc - 4));
        XR_FAULT
    } else if n1 == n0 && got == XTLB_MAGIC {
        line(format_args!("smp: xtlb cpu1 read after unmap returned the old value"));
        XR_LEAK
    } else {
        line(format_args!("smp: xtlb cpu1 other n={} esr={esr:#x}", n1 - n0));
        XR_OTHER
    };
    block.xtlb_result.store(res, Ordering::SeqCst);
    block.xtlb_phase.store(X_DONE, Ordering::SeqCst);
}

/// Core 0 side. Prints `smp: xtlb unmap cpu0 → cpu1 fault` only when core 1
/// faulted; `smp: xtlb leak` when core 1 still read the unmapped page.
fn cpu0_xtlb(b: &Block) -> bool {
    let freq = cntfrq().max(1);
    let page_va = core::ptr::addr_of_mut!(XTLB_PAGE) as u64;
    unsafe { core::ptr::write_volatile(page_va as *mut u64, XTLB_MAGIC) };
    let pa = crate::paging::identity_pa(page_va);
    let mut va = 0;
    for i in (0..512u64).rev() {
        let cand = crate::paging::MAP_WINDOW + i * 4096;
        if crate::paging::map_page(cand, pa) {
            va = cand;
            break;
        }
    }
    if va == 0 {
        line(format_args!("smp: xtlb missed (no free window slot)"));
        return false;
    }
    b.xtlb_va.store(va, Ordering::SeqCst);
    b.xtlb_phase.store(X_MAPPED, Ordering::SeqCst);
    if !wait_phase(b, X_READ1, freq * 2) {
        line(format_args!("smp: xtlb missed (cpu1 did not read)"));
        return false;
    }
    let unmapped = crate::paging::unmap_page(va);
    b.xtlb_phase.store(X_UNMAPPED, Ordering::SeqCst);
    if !unmapped || !wait_phase(b, X_DONE, freq * 2) {
        line(format_args!("smp: xtlb missed (unmap={unmapped} or cpu1 timeout)"));
        return false;
    }
    match b.xtlb_result.load(Ordering::SeqCst) {
        XR_FAULT => {
            line(format_args!("smp: xtlb unmap cpu0 → cpu1 fault"));
            true
        }
        XR_LEAK => {
            line(format_args!("smp: xtlb leak"));
            false
        }
        r => {
            line(format_args!("smp: xtlb missed result={r}"));
            false
        }
    }
}

/// S1 + S2 probe on core 0. Prints the S1 lines, then waits for the
/// secondary's S2 verdict and prints `smp: ok cpu1 s2 …` or a miss line.
pub fn observe_probe() -> bool {
    let mut w = crate::uart::raw();
    let _ = writeln!(w, "smp: cpu0 mpidr={:#018x}", mpidr());
    let b = unsafe { &mut *core::ptr::addr_of_mut!(BLOCK) };
    b.mpidr.store(NOT_SEEN, Ordering::SeqCst);
    b.verdict.store(V_NONE, Ordering::SeqCst);
    b.ticks.store(0, Ordering::SeqCst);
    b.go.store(0, Ordering::SeqCst);
    b.mair = sysreg!("mair_el1");
    b.tcr = sysreg!("tcr_el1");
    b.ttbr0 = sysreg!("ttbr0_el1");
    b.ttbr1 = sysreg!("ttbr1_el1");
    // Core 0's SCTLR with SPAN back at its reset value: cpu1 clears it itself.
    b.sctlr = sysreg!("sctlr_el1") | SCTLR_SPAN;
    let stack_lo = core::ptr::addr_of!(SECONDARY_STACK) as u64;
    b.sp = crate::paging::to_high_va(stack_lo + 16 * 1024);
    b.entry = crate::paging::to_high_va(cpu1_main as *const () as usize as u64);
    let block_va = b as *const Block as u64;
    b.arg = crate::paging::to_high_va(block_va);
    unsafe { asm!("dsb sy", options(nostack)) };
    // The secondary reads the block MMU off (non-cacheable): push to PoC.
    let mut a = block_va as usize & !63;
    while a < block_va as usize + core::mem::size_of::<Block>() {
        clean_inval(a);
        a += 64;
    }
    let flag_va = block_va as usize;
    let entry_pa = crate::paging::identity_pa(_secondary_entry as *const () as usize as u64);
    let block_pa = crate::paging::identity_pa(block_va);
    let rc = psci_cpu_on(TARGET_MPIDR, entry_pa, block_pa);
    // Do not print while the secondary may be writing its raw MMU-off line.
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
    let t1 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t1) < freq / 20 {
        core::hint::spin_loop();
    }
    // From here both cores may print: console lock only.
    line(format_args!("smp: cpu_on target={TARGET_MPIDR:#x} rc={rc}"));
    if rc != 0 || seen & 0xff_00ff_ffff != TARGET_MPIDR {
        line(format_args!("smp: cpu1 missed seen={seen:#x}"));
        return false;
    }
    line(format_args!("smp: ok cpu1 mpidr={seen:#018x} up"));
    b.go.store(1, Ordering::SeqCst);
    for i in 0..TICKS {
        line(format_args!("smp: tick cpu0 {i}"));
    }
    let t2 = crate::timer::cntpct();
    let mut v = V_NONE;
    while crate::timer::cntpct().wrapping_sub(t2) < freq * 2 {
        v = b.verdict.load(Ordering::SeqCst);
        if v != V_NONE {
            break;
        }
        core::hint::spin_loop();
    }
    let bad = CTOS_CPU1_EXC[4].load(Ordering::SeqCst);
    let ticks = b.ticks.load(Ordering::SeqCst);
    // Core 0 re-checks what the secondary's handler recorded.
    let esr = CTOS_CPU1_EXC[0].load(Ordering::SeqCst);
    let n = CTOS_CPU1_EXC[3].load(Ordering::SeqCst);
    if v == V_OK && bad == 0 && ticks == TICKS && esr == BRK_ESR && n == 1 {
        line(format_args!("smp: ok cpu1 s2 mmu-on pan=1 span=0 vbar=own exc=handled console=locked"));
        cpu0_xtlb(b)
    } else {
        line(format_args!("smp: cpu1 s2 missed verdict={v} bad={bad:#x} ticks={ticks}"));
        false
    }
}
