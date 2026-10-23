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
//! S3 (ADR-103): cross-core unmap probe. S4 (ADR-104): core 1 then enters
//! the kernel's normal mode (PerCpu in `TPIDR_EL1`, kernel vectors, own
//! stacks, own GIC CPU interface + timer) and runs the EL0 isolation probes
//! while core 0 waits; then both cores run the ASID-generation probes.
//!
//! S5 (ADR-105): after S4, both cores pick tasks from a locked run queue and
//! run EL0 concurrently (private TaskSpace + allocator ASID each); a mid-run
//! ASID rollover keeps core 1's running ASID reserved.
//!
//! Not a general-purpose SMP OS. No preemption, no drivers on core 1. The
//! default build has none of this code and QEMU runs one core (ADR-091 rev 4
//! scope is unchanged).

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
    // ADR-104 (S4). Written with the MMU on only.
    s4_go: AtomicU64,
    s4_phase: AtomicU64,
    s4_kttbr0: AtomicU64,
    s4_span: AtomicU64,
    s4_va: AtomicU64,
    s4_asid: AtomicU64,
    s4_gen: AtomicU64,
    s4_result: AtomicU64,
    s4_probes: AtomicU64,
    s4_failed: AtomicU64,
    s5_go: AtomicU64,
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
    s4_go: AtomicU64::new(0),
    s4_phase: AtomicU64::new(0),
    s4_kttbr0: AtomicU64::new(0),
    s4_span: AtomicU64::new(0),
    s4_va: AtomicU64::new(0),
    s4_asid: AtomicU64::new(0),
    s4_gen: AtomicU64::new(0),
    s4_result: AtomicU64::new(0),
    s4_probes: AtomicU64::new(0),
    s4_failed: AtomicU64::new(0),
    s5_go: AtomicU64::new(0),
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
        // ADR-104 (S4): wait for core 0, then enter the kernel's normal mode.
        while block.s4_go.load(Ordering::SeqCst) == 0 {
            core::hint::spin_loop();
        }
        cpu1_enter_kernel_mode(block);
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

// ---- ADR-104 (S4): core 1 in the kernel's normal mode -------------------
//
// After S3, core 1 installs PerCpu[1] in TPIDR_EL1, takes core 0's
// SCTLR.SPAN, the **kernel** vector table, its own SP_EL1 exception stack
// and its own SP_EL0 thread stack (SPSel = 0, EL1t like core 0's kernel
// threads), and runs the EL0 isolation probes there, one at a time, while
// core 0 waits. Then the ASID-generation probes run on both cores.

/// Core 1's thread stack (SP_EL0), exception stack (SP_EL1) and fatal
/// stack (ADR-105), each with a 4 KiB guard page below it. The guard pages
/// are unmapped at boot with the linker-stack guards (identity), before the
/// TTBR1 RAM clone, so the high alias has the same holes.
#[repr(C, align(4096))]
struct Guarded<const N: usize> {
    guard: [u8; 4096],
    stack: [u8; N],
}
const THREAD_SZ: usize = 64 * 1024;
const EXC_SZ: usize = 16 * 1024;
const FATAL_SZ: usize = 8 * 1024;
static mut CPU1_THREAD_STACK: Guarded<THREAD_SZ> = Guarded { guard: [0; 4096], stack: [0; THREAD_SZ] };
static mut CPU1_EXC_STACK: Guarded<EXC_SZ> = Guarded { guard: [0; 4096], stack: [0; EXC_SZ] };
static mut CPU1_FATAL_STACK: Guarded<FATAL_SZ> = Guarded { guard: [0; 4096], stack: [0; FATAL_SZ] };

fn high(p: u64) -> u64 {
    crate::paging::to_high_va(crate::paging::identity_pa(p))
}

fn guard_va<const N: usize>(g: *const Guarded<N>) -> u64 {
    g as u64
}

/// Identity VAs of core 1's three guard pages (boot-time unmap).
pub fn cpu1_stack_guards() -> [u64; 3] {
    [
        crate::paging::identity_pa(guard_va(core::ptr::addr_of!(CPU1_THREAD_STACK))),
        crate::paging::identity_pa(guard_va(core::ptr::addr_of!(CPU1_EXC_STACK))),
        crate::paging::identity_pa(guard_va(core::ptr::addr_of!(CPU1_FATAL_STACK))),
    ]
}

fn cpu1_thread_range() -> (u64, u64) {
    let lo = high(core::ptr::addr_of!(CPU1_THREAD_STACK) as u64) + 4096;
    (lo, lo + THREAD_SZ as u64)
}

fn cpu1_exc_top() -> u64 {
    high(core::ptr::addr_of!(CPU1_EXC_STACK) as u64) + 4096 + EXC_SZ as u64
}

fn cpu1_fatal_range() -> (u64, u64) {
    let lo = high(core::ptr::addr_of!(CPU1_FATAL_STACK) as u64) + 4096;
    (lo, lo + FATAL_SZ as u64)
}

/// `sp` lies in core 1's own fatal stack (ADR-105).
pub fn on_cpu1_fatal_stack(sp: u64) -> bool {
    let (lo, hi) = cpu1_fatal_range();
    sp > lo && sp <= hi
}

static CPU1_FATAL_SEEN: AtomicU64 = AtomicU64::new(0);

/// Core 1's fatal handler ran (1 = on its own stack, 2 = another stack).
pub fn note_cpu1_fatal(own: bool) {
    CPU1_FATAL_SEEN.store(if own { 1 } else { 2 }, Ordering::SeqCst);
}

fn kernel_vbar() -> u64 {
    crate::paging::to_high_va(crate::exception::vector_table_addr())
}

fn cpu1_enter_kernel_mode(block: &'static Block) -> ! {
    let kttbr0 = block.s4_kttbr0.load(Ordering::SeqCst);
    // ADR-105: core 1's own fatal stack, used by `fatal_enter` via PerCpu.
    crate::percpu::set_fatal_top(1, cpu1_fatal_range().1);
    let _ = crate::percpu::install(1, kttbr0);
    let span = block.s4_span.load(Ordering::SeqCst) & SCTLR_SPAN;
    let sctlr = (sysreg!("sctlr_el1") & !SCTLR_SPAN) | span;
    let (_, thr_top) = cpu1_thread_range();
    let exc_top = cpu1_exc_top();
    let vbar = kernel_vbar();
    let entry = cpu1_kernel_main as *const () as usize as u64;
    let arg = block as *const Block as u64;
    unsafe {
        asm!(
            "msr sctlr_el1, {sctlr}",
            "isb",
            "msr vbar_el1, {v}",
            "isb",
            // SPSel is 1 here: `sp` is SP_EL1. Own exception stack, then the
            // thread stack in SP_EL0, then run on SP_EL0 (EL1t).
            "msr sp_el0, {t}",
            "mov sp, {e}",
            "msr spsel, #0",
            "isb",
            "br {f}",
            sctlr = in(reg) sctlr,
            v = in(reg) vbar,
            t = in(reg) thr_top,
            e = in(reg) exc_top,
            f = in(reg) entry,
            in("x0") arg,
            options(noreturn),
        );
    }
}

const S4_PROBES: u64 = 13;

/// Core 1 kernel-mode main (EL1t, own stacks, kernel vectors).
extern "C" fn cpu1_kernel_main(block: &'static Block) -> ! {
    let tp = crate::percpu::this().map(|b| b as *const _ as u64).unwrap_or(0);
    let id = crate::percpu::id();
    let vbar_ok = sysreg!("vbar_el1") == kernel_vbar();
    let spsel = sysreg!("spsel") & 1;
    let sp: u64;
    unsafe { asm!("mov {s}, sp", s = out(reg) sp, options(nomem, nostack)) };
    let (lo, hi) = cpu1_thread_range();
    let own_sp = sp > lo && sp <= hi;
    let span = (sysreg!("sctlr_el1") & SCTLR_SPAN) >> 23;
    let pan = pstate_pan();
    let mode_ok = tp != 0 && id == 1 && vbar_ok && spsel == 0 && own_sp && pan == 1;
    line(format_args!(
        "smp: cpu1 kernel-mode tpidr={} vbar={} spsel={spsel} sp={} pan={pan} span={span} cpu={id}",
        if tp != 0 && id == 1 { "percpu1" } else { "missing" },
        if vbar_ok { "kernel" } else { "other" },
        if own_sp { "own-thread" } else { "other" },
    ));
    let mut failed = 0u64;
    if mode_ok {
        // Core 1's own (banked) GIC CPU interface, timer PPI and CNTP, as
        // the single-core boot has before its probes. DAIF stays masked at
        // EL1; the EL0 probes that enter with IRQ/FIQ clear take core 1's
        // timer through the lower-EL trampolines.
        crate::gic::init_this_cpu();
        crate::timer::init();
        failed = cpu1_run_probes(block);
        crate::timer::stop();
    }
    block.s4_failed.store(if mode_ok { failed } else { u64::MAX }, Ordering::SeqCst);
    block.s4_phase.store(P_PROBES_DONE, Ordering::SeqCst);
    if mode_ok {
        cpu1_asidgen(block);
        // ADR-105 (S5): wait for core 0, then run concurrent EL0.
        while block.s5_go.load(Ordering::SeqCst) == 0 {
            core::hint::spin_loop();
        }
        cpu1_s5_entry();
    }
    loop {
        unsafe { asm!("wfe", options(nomem, nostack)) };
    }
}

fn tag(block: &Block, name: &str, ok: bool) -> u64 {
    block.s4_probes.fetch_add(1, Ordering::SeqCst);
    line(format_args!(
        "smp: cpu1 probe {name} {} cpu={}",
        if ok { "ok" } else { "missed" },
        crate::percpu::id()
    ));
    if ok { 0 } else { 1 }
}

/// The single-core boot's EL0 isolation probes, in the same order, on core 1.
/// Returns the number that missed. The standing task (needs the GIC timer)
/// and the non-isolation samples are not part of S4.
fn cpu1_run_probes(b: &Block) -> u64 {
    let mut f = 0;
    f += tag(b, "el0", crate::el0::observe_probe());
    f += tag(b, "svc", crate::syscall::observe_probe());
    f += tag(b, "sys-ptr", crate::syscall::observe_sys_ptr_probe());
    f += tag(b, "libctos", crate::libctos::observe_probe());
    f += tag(b, "loader", crate::loader::observe_probe());
    let pan_after = crate::pan::observe_after_el0();
    f += tag(b, "pan-held", crate::pan::observe_held(pan_after));
    f += tag(b, "asid", crate::asid::observe_probe());
    f += tag(b, "t2t", crate::t2t::observe_probe());
    f += tag(b, "xo", crate::xo::observe_probe());
    f += tag(b, "ttbr1", crate::ttbr1::observe_probe());
    f += tag(b, "ident", crate::teardown::observe_probe());
    f += tag(b, "el0-reach", crate::reach::observe_probe());
    f += tag(b, "pan", crate::pan::pan_probe_ready());
    f
}

// ASID-generation probes. Phases: core 1 writes even, core 0 writes odd.
const P_PROBES_DONE: u64 = 2;
const P_ALLOC_GO: u64 = 3;
const P_ALLOC_DONE: u64 = 4;
const P_MAPPED: u64 = 5;
const P_READ1: u64 = 6;
const P_REISSUED: u64 = 7;
const P_DONE: u64 = 8;

/// Each core allocates this many; 2 × 150 > 253 forces at least one rollover.
const ALLOC_N: usize = 150;
static CPU1_ALLOCS: [AtomicU64; ALLOC_N] = [const { AtomicU64::new(0) }; ALLOC_N];

const ASIDGEN_MAGIC: u64 = 0x4153_4944_4745_4e04; // "ASIDGEN" + 4
const ASIDGEN_SENTINEL: u64 = 0xdead_0000_0000_0004;
const AR_FAULT: u64 = 1;
const AR_LEAK: u64 = 2;
const AR_FIRST_READ_BAD: u64 = 3;
const AR_OTHER: u64 = 4;

fn wait_s4(b: &Block, want: u64, ticks: u64) -> bool {
    let t0 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t0) < ticks {
        if b.s4_phase.load(Ordering::SeqCst) == want {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

fn enc(s: crate::asidgen::Slot) -> u64 {
    (s.gen << 8) | s.asid
}

fn cpu1_asidgen(b: &'static Block) {
    let freq = cntfrq().max(1);
    // (1) Concurrent allocation from both cores.
    if !wait_s4(b, P_ALLOC_GO, freq * 4) {
        return;
    }
    for slot in CPU1_ALLOCS.iter() {
        slot.store(enc(crate::asidgen::alloc()), Ordering::SeqCst);
    }
    b.s4_phase.store(P_ALLOC_DONE, Ordering::SeqCst);
    // (2) Reissue after rollover. Task T1 on core 1 reads the nG page.
    if !wait_s4(b, P_MAPPED, freq * 4) {
        return;
    }
    let va = b.s4_va.load(Ordering::SeqCst);
    let kttbr0 = crate::paging::kernel_ttbr0();
    let mut t1 = crate::asidgen::Slot::NONE;
    let asid = crate::asidgen::switch_to(&mut t1);
    crate::paging::switch_ttbr0_no_tlbi(kttbr0 | (asid << 48));
    let first = unsafe { core::ptr::read_volatile(va as *const u64) };
    // T1 exits. Core 1 stays on that TTBR0 value (lazy; TCG flushes its
    // TLB on any ASID change, which would hide the stale entry).
    crate::asidgen::release_cpu();
    b.s4_asid.store(asid, Ordering::SeqCst);
    b.s4_gen.store(t1.gen, Ordering::SeqCst);
    b.s4_phase.store(P_READ1, Ordering::SeqCst);
    if !wait_s4(b, P_REISSUED, freq * 4) {
        crate::paging::switch_ttbr0_no_tlbi(kttbr0);
        return;
    }
    // Task T2 got the same ASID in a later generation (allocated by core 0).
    let g2 = b.s4_gen.load(Ordering::SeqCst);
    let mut t2 = crate::asidgen::Slot { gen: g2, asid };
    let a2 = crate::asidgen::switch_to(&mut t2);
    crate::paging::switch_ttbr0_no_tlbi(kttbr0 | (a2 << 48));
    crate::exception::arm_dabort_va(va);
    let mut got = ASIDGEN_SENTINEL;
    unsafe {
        asm!("ldr {v}, [{a}]", v = inout(reg) got, a = in(reg) va, options(nostack));
    }
    let caught = crate::exception::dabort_va_caught();
    crate::asidgen::release_cpu();
    crate::paging::switch_ttbr0_no_tlbi(kttbr0);
    let seen = crate::percpu::this().map(|p| p.asid_gen_seen.load(Ordering::SeqCst)).unwrap_or(0);
    let ec = (caught >> 26) & 0x3f;
    let dfsc = caught & 0x3f;
    let res = if first != ASIDGEN_MAGIC {
        line(format_args!("smp: asidgen cpu1 first read bad"));
        AR_FIRST_READ_BAD
    } else if a2 != asid || t2.gen != g2 {
        line(format_args!("smp: asidgen cpu1 slot changed"));
        AR_OTHER
    } else if caught != 0 && ec == 0x25 && (0x04..=0x07).contains(&dfsc) && got == ASIDGEN_SENTINEL {
        line(format_args!(
            "smp: asidgen cpu1 fault ec=0x25 dfsc=trans-l{} gen-seen={seen} cpu=1",
            dfsc - 4
        ));
        AR_FAULT
    } else if caught == 0 && got == ASIDGEN_MAGIC {
        line(format_args!("smp: asidgen cpu1 read the retired page under the reissued asid cpu=1"));
        AR_LEAK
    } else {
        line(format_args!("smp: asidgen cpu1 other caught={caught:#x}"));
        AR_OTHER
    };
    b.s4_result.store(res, Ordering::SeqCst);
    b.s4_phase.store(P_DONE, Ordering::SeqCst);
}

fn cpu0_alloc_test(b: &Block) -> bool {
    let freq = cntfrq().max(1);
    let r0 = crate::asidgen::rollovers();
    let mut mine = [0u64; ALLOC_N];
    b.s4_phase.store(P_ALLOC_GO, Ordering::SeqCst);
    for m in mine.iter_mut() {
        *m = enc(crate::asidgen::alloc());
    }
    if !wait_s4(b, P_ALLOC_DONE, freq * 4) {
        line(format_args!("smp: asidgen alloc missed (cpu1 timeout)"));
        return false;
    }
    let mut all = [0u64; 2 * ALLOC_N];
    all[..ALLOC_N].copy_from_slice(&mine);
    for (i, s) in CPU1_ALLOCS.iter().enumerate() {
        all[ALLOC_N + i] = s.load(Ordering::SeqCst);
    }
    let range_ok = all
        .iter()
        .all(|&e| (crate::asidgen::FIRST..=crate::asidgen::LAST).contains(&(e & 0xff)) && e >> 8 != 0);
    all.sort_unstable();
    let unique = all.windows(2).all(|w| w[0] != w[1]);
    let gens = 1 + all.windows(2).filter(|w| w[0] >> 8 != w[1] >> 8).count();
    let rolled = crate::asidgen::rollovers() - r0;
    let ok = range_ok && unique && rolled >= 1 && gens >= 2;
    line(format_args!(
        "smp: asidgen alloc cpu0+cpu1 n={} {} range={} gens={gens} rollovers={rolled}",
        2 * ALLOC_N,
        if unique { "unique" } else { "duplicate" },
        if range_ok { "3..=255" } else { "bad" },
    ));
    ok
}

fn cpu0_reissue(b: &Block) -> bool {
    let freq = cntfrq().max(1);
    let Some(pa) = crate::frame::alloc() else {
        line(format_args!("smp: asidgen missed (no frame)"));
        return false;
    };
    let cpu = crate::paging::frame_cpu_va(pa);
    unsafe { core::ptr::write_volatile(cpu as *mut u64, ASIDGEN_MAGIC) };
    unsafe { asm!("dc civac, {x}", "dsb ish", x = in(reg) cpu, options(nostack)) };
    let mut va = 0;
    for i in (0..512u64).rev() {
        let cand = crate::paging::MAP_WINDOW + i * 4096;
        if crate::paging::map_page_ng(cand, pa) {
            va = cand;
            break;
        }
    }
    if va == 0 {
        crate::frame::free(pa);
        line(format_args!("smp: asidgen missed (no free window slot)"));
        return false;
    }
    b.s4_va.store(va, Ordering::SeqCst);
    b.s4_phase.store(P_MAPPED, Ordering::SeqCst);
    if !wait_s4(b, P_READ1, freq * 4) {
        line(format_args!("smp: asidgen missed (cpu1 did not read)"));
        return false;
    }
    let asid = b.s4_asid.load(Ordering::SeqCst);
    let g1 = b.s4_gen.load(Ordering::SeqCst);
    // T1 is retired: tear its mapping down with no TLBI (the generation rule
    // defers that to the rollover flush), then allocate until the same ASID
    // is issued again in a later generation.
    let cleared = crate::paging::clear_window_no_tlbi(va);
    let mut t2 = crate::asidgen::Slot::NONE;
    let mut tries = 0u64;
    while tries < 4 * crate::asidgen::PER_GEN {
        let s = crate::asidgen::alloc();
        tries += 1;
        if s.gen > g1 && s.asid == asid {
            t2 = s;
            break;
        }
    }
    if !cleared || t2.gen == 0 {
        line(format_args!("smp: asidgen missed (cleared={cleared} reissued={})", t2.gen != 0));
        return false;
    }
    line(format_args!(
        "smp: asidgen rollover gen {g1}->{} asid {asid} reissued (flush={})",
        t2.gen,
        if cfg!(feature = "asid-rollover-local-probe") { "local" } else { "vmalle1is" }
    ));
    b.s4_gen.store(t2.gen, Ordering::SeqCst);
    b.s4_phase.store(P_REISSUED, Ordering::SeqCst);
    let done = wait_s4(b, P_DONE, freq * 4);
    crate::paging::invalidate_va(va);
    crate::frame::free(pa);
    if !done {
        line(format_args!("smp: asidgen missed (cpu1 timeout)"));
        return false;
    }
    match b.s4_result.load(Ordering::SeqCst) {
        AR_FAULT => {
            line(format_args!("smp: asidgen reissue cpu0 rollover → cpu1 fault"));
            true
        }
        AR_LEAK => {
            line(format_args!("smp: asidgen leak"));
            false
        }
        r => {
            line(format_args!("smp: asidgen missed result={r}"));
            false
        }
    }
}

/// ADR-104 (S4) on core 0, after S1–S3 passed and core 0 ran the driver /
/// FAT / scheduler / GIC distributor init (no timer on core 0). Prints `smp: ok cpu1 s4 …` only
/// when core 1 entered kernel mode, every probe passed on core 1, the
/// concurrent allocation was unique and the reissue probe faulted on core 1.
pub fn observe_s4() -> bool {
    let b = unsafe { &*core::ptr::addr_of!(BLOCK) };
    let freq = cntfrq().max(1);
    b.s4_kttbr0.store(crate::paging::kernel_ttbr0(), Ordering::SeqCst);
    b.s4_span.store(sysreg!("sctlr_el1") & SCTLR_SPAN, Ordering::SeqCst);
    b.s4_phase.store(0, Ordering::SeqCst);
    line(format_args!("smp: s4 go cpu1"));
    b.s4_go.store(1, Ordering::SeqCst);
    unsafe { asm!("dsb ish", "sev", options(nostack)) };
    // Core 1 runs the probes; core 0 only waits (no output, no EL0).
    if !wait_s4(b, P_PROBES_DONE, freq * 60) {
        line(format_args!("smp: s4 missed (cpu1 probes timeout)"));
        return false;
    }
    let failed = b.s4_failed.load(Ordering::SeqCst);
    let n = b.s4_probes.load(Ordering::SeqCst);
    let probes_ok = failed == 0 && n == S4_PROBES;
    if probes_ok {
        line(format_args!("smp: cpu1 probes ok n={n} cpu=1"));
    } else {
        line(format_args!("smp: cpu1 probes missed failed={failed} n={n}"));
        if failed == u64::MAX {
            return false;
        }
    }
    let alloc_ok = cpu0_alloc_test(b);
    let reissue_ok = cpu0_reissue(b);
    let ok = probes_ok && alloc_ok && reissue_ok;
    if ok {
        line(format_args!(
            "smp: ok cpu1 s4 percpu=tpidr el0=cpu1 probes={n} asidgen=unique,rollover-is"
        ));
    } else {
        line(format_args!(
            "smp: s4 missed probes={probes_ok} alloc={alloc_ok} reissue={reissue_ok}"
        ));
    }
    ok
}


// ---- ADR-105 (S5): concurrent EL0 on both cores -----------------------
//
// Two tasks, each with its own TaskSpace (allocator ASID + private tables)
// and its own EL0 text / data / stack. Both cores pick tasks from the
// shared run queue and drive EL0 concurrently. Mid-run core 0 forces an
// ASID rollover while core 1 is still running; the reserved-running-ASID
// rule keeps core 1's ASID. A reach / t2t / xo check runs against both
// spaces. Negative build `asid-reserve-drop-probe` drops the reserved
// rule and must report that core 1's ASID was reissued while it ran.

const S5_MAGIC_A: u64 = 0x4153_5f53_3500_0001;
const S5_MAGIC_B: u64 = 0x4153_5f53_3500_0002;
const SVC_EXIT_A64: u32 = 0xD4000001 | (16 << 5);
#[allow(dead_code)] const MOVZ_X0: u32 = 0xD2800000;
#[allow(dead_code)] const MOVK_X0_16: u32 = 0xF2A00000;
#[allow(dead_code)] const LDR_X1_PC16: u32 = 0x58000081; // LDR X1, [PC, #16]
const STR_X1_X0: u32 = 0xF9000001;
const LDR_X2_X0: u32 = 0xF9400002;
#[allow(dead_code)] const BRK0: u32 = 0xD4200000;
const LDR_X0_X1: u32 = 0xF9400020; // LDR X0, [X1]
#[allow(dead_code)] const MOVZ_X1: u32 = 0xD2800001;
#[allow(dead_code)] const MOVK_X1_16: u32 = 0xF2A00001;

#[repr(C, align(4096))]
struct TaskPage([u8; 4096]);

#[allow(dead_code)]
struct S5Task {
    id: usize,
    space: crate::paging::TaskSpace,
    slot: crate::asidgen::Slot,
    text: u64,
    data: u64,
    stack: u64,
    magic: u64,
}

static mut S5_TEXT: [TaskPage; 2] = [TaskPage([0; 4096]), TaskPage([0; 4096])];
static mut S5_DATA: [TaskPage; 2] = [TaskPage([0; 4096]), TaskPage([0; 4096])];
static mut S5_STACK: [TaskPage; 2] = [TaskPage([0; 4096]), TaskPage([0; 4096])];
static mut S5_TASKS: [Option<S5Task>; 2] = [None, None];

static S5_READY: AtomicU64 = AtomicU64::new(0);
static S5_CPU0_RUNS: AtomicU64 = AtomicU64::new(0);
static S5_CPU1_RUNS: AtomicU64 = AtomicU64::new(0);
static S5_CPU1_ASID: AtomicU64 = AtomicU64::new(0);
static S5_CPU1_GEN: AtomicU64 = AtomicU64::new(0);
static S5_CPU1_HOLD: AtomicU64 = AtomicU64::new(0);
static S5_ROLLOVER_DONE: AtomicU64 = AtomicU64::new(0);
static S5_REISSUED: AtomicU64 = AtomicU64::new(0);
static S5_RESULT: AtomicU64 = AtomicU64::new(0);

fn write_u32(pa: u64, i: usize, v: u32) {
    let va = crate::paging::frame_cpu_va(pa) + (i * 4) as u64;
    unsafe { core::ptr::write_volatile(va as *mut u32, v) };
}

fn sync_icache_pa(pa: u64) {
    let va = crate::paging::frame_cpu_va(pa);
    unsafe {
        asm!(
            "dc cvau, {x}", "dsb ish", "ic ivau, {x}", "dsb ish", "isb",
            x = in(reg) va, options(nostack),
        );
    }
}

/// EL0 program: materialise `magic` in X1 via MOVZ/MOVK (no literal pool —
/// the text page is execute-only), store it at DATA, reload, then `SVC #16`.
fn fill_text(pa: u64, magic: u64) {
    let dva = crate::paging::TASK_DATA_VA;
    let movz = |rd: u32, imm: u32, hw: u32| 0xD2800000 | (hw << 21) | (imm << 5) | rd;
    let movk = |rd: u32, imm: u32, hw: u32| 0xF2800000 | (hw << 21) | (imm << 5) | rd;
    write_u32(pa, 0, movz(0, (dva as u32) & 0xffff, 0));
    write_u32(pa, 1, movk(0, ((dva >> 16) as u32) & 0xffff, 1));
    write_u32(pa, 2, movz(1, (magic as u32) & 0xffff, 0));
    write_u32(pa, 3, movk(1, ((magic >> 16) as u32) & 0xffff, 1));
    write_u32(pa, 4, movk(1, ((magic >> 32) as u32) & 0xffff, 2));
    write_u32(pa, 5, movk(1, ((magic >> 48) as u32) & 0xffff, 3));
    write_u32(pa, 6, STR_X1_X0);
    write_u32(pa, 7, LDR_X2_X0);
    write_u32(pa, 8, SVC_EXIT_A64);
    sync_icache_pa(pa);
}

fn make_task(i: usize, magic: u64) -> Option<S5Task> {
    let space = crate::paging::task_space_new()?;
    let text = crate::paging::identity_pa(unsafe { core::ptr::addr_of!(S5_TEXT[i]) as u64 });
    let data = crate::paging::identity_pa(unsafe { core::ptr::addr_of!(S5_DATA[i]) as u64 });
    let stack = crate::paging::identity_pa(unsafe { core::ptr::addr_of!(S5_STACK[i]) as u64 });
    fill_text(text, magic);
    unsafe {
        core::ptr::write_bytes(crate::paging::frame_cpu_va(data) as *mut u8, 0, 4096);
        core::ptr::write_bytes(crate::paging::frame_cpu_va(stack) as *mut u8, 0, 4096);
    }
    let leaf = if cfg!(feature = "xo-leak-probe") {
        crate::paging::TaskLeaf::Rx
    } else {
        crate::paging::TaskLeaf::Xo
    };
    if !crate::paging::task_space_map(&space, 0, text, leaf)
        || !crate::paging::task_space_map(&space, 1, data, crate::paging::TaskLeaf::Rw)
        || !crate::paging::task_space_map(&space, 2, stack, crate::paging::TaskLeaf::Rw)
    {
        crate::paging::task_space_free(&space);
        return None;
    }
    let id = match crate::runq::spawn() {
        Some(id) => id,
        None => {
            crate::paging::task_space_free(&space);
            return None;
        }
    };
    let slot = crate::asidgen::alloc();
    Some(S5Task { id, space, slot, text, data, stack, magic })
}

fn s5_run_once(t: &mut S5Task) -> bool {
    let asid = crate::asidgen::switch_to(&mut t.slot);
    let ttbr = crate::paging::task_ttbr0(&t.space, asid);
    crate::el0::install_task(
        crate::paging::TASK_TEXT_VA,
        crate::paging::TASK_STACK_VA + 4096,
        ttbr,
    );
    if !crate::el0::is_active() {
        crate::asidgen::release_cpu();
        return false;
    }
    unsafe {
        crate::exception::eret_to_el0_masked_ttbr(
            crate::paging::TASK_TEXT_VA,
            0,
            crate::paging::TASK_STACK_VA + 4096,
            ttbr,
        );
    }
    let exited = !crate::el0::is_active();
    let got = unsafe { core::ptr::read_volatile(crate::paging::frame_cpu_va(t.data) as *const u64) };
    crate::asidgen::release_cpu();
    crate::paging::switch_ttbr0_no_tlbi(crate::paging::kernel_ttbr0());
    exited && got == t.magic
}

fn s5_check_spaces(a: &S5Task, b: &S5Task) -> bool {
    let mut ra = [crate::paging::TaskReach::default(); 8];
    let mut rb = [crate::paging::TaskReach::default(); 8];
    let na = crate::paging::task_space_walk(&a.space, &mut ra);
    let nb = crate::paging::task_space_walk(&b.space, &mut rb);
    let want_xo = !cfg!(feature = "xo-leak-probe");
    let mut ok = true;
    for (n, r, name) in [(na, &ra[..], "a"), (nb, &rb[..], "b")] {
        if n != 3 {
            line(format_args!("smp: s5 reach {name} pages={n} (want 3)"));
            ok = false;
            continue;
        }
        let text = r.iter().find(|p| p.va == crate::paging::TASK_TEXT_VA);
        let data = r.iter().find(|p| p.va == crate::paging::TASK_DATA_VA);
        let stack = r.iter().find(|p| p.va == crate::paging::TASK_STACK_VA);
        match (text, data, stack) {
            (Some(t), Some(d), Some(s)) => {
                let xo_ok = if want_xo { t.x && !t.r && !t.w } else { t.x && t.r };
                if !(xo_ok && d.r && d.w && !d.x && s.r && s.w && !s.x) {
                    line(format_args!(
                        "smp: s5 reach {name} bad text=r{}w{}x{} data=r{}w{}x{}",
                        t.r as u8, t.w as u8, t.x as u8, d.r as u8, d.w as u8, d.x as u8
                    ));
                    ok = false;
                }
            }
            _ => {
                line(format_args!("smp: s5 reach {name} missing leaf"));
                ok = false;
            }
        }
    }
    if rb[..nb.min(8)].iter().any(|p| p.pa == a.data) || ra[..na.min(8)].iter().any(|p| p.pa == b.data)
    {
        line(format_args!("smp: s5 t2t shared frame"));
        ok = false;
    }
    ok
}

/// Core 1's three guard pages are unmapped (identity) and their high
/// aliases are unmapped too (the TTBR1 RAM clone ran after the punch).
fn s5_check_guards() -> bool {
    let mut ok = true;
    for (i, g) in cpu1_stack_guards().into_iter().enumerate() {
        let name = ["thread", "exc", "fatal"][i];
        let id_mapped = crate::paging::is_mapped(g);
        let hi = crate::paging::to_high_va(g);
        let hi_mapped = crate::paging::high_mapped(hi);
        if id_mapped || hi_mapped {
            line(format_args!("smp: s5 guard {name} still mapped id={id_mapped} high={hi_mapped}"));
            ok = false;
        }
    }
    if ok {
        line(format_args!("smp: s5 guards unmapped thread,exc,fatal cpu=1"));
    }
    ok
}

/// Live xo: EL0 load of this task's own text must fault (unless leak).
fn s5_xo_live(t: &mut S5Task) -> bool {
    let Some(pa) = crate::frame::alloc() else {
        return false;
    };
    let text_va = crate::paging::TASK_TEXT_VA;
    write_u32(pa, 0, MOVZ_X1 | ((text_va as u32 & 0xffff) << 5));
    write_u32(pa, 1, MOVK_X1_16 | (((text_va >> 16) as u32 & 0xffff) << 5));
    write_u32(pa, 2, LDR_X0_X1);
    write_u32(pa, 3, SVC_EXIT_A64);
    sync_icache_pa(pa);
    if !crate::paging::task_space_map(&t.space, 3, pa, crate::paging::TaskLeaf::Xo) {
        crate::frame::free(pa);
        return false;
    }
    let tramp = crate::paging::MAP_WINDOW + 3 * 4096;
    let asid = crate::asidgen::switch_to(&mut t.slot);
    let ttbr = crate::paging::task_ttbr0(&t.space, asid);
    crate::exception::arm_el0_capture();
    crate::el0::install_task(tramp, crate::paging::TASK_STACK_VA + 4096, ttbr);
    unsafe {
        crate::exception::eret_to_el0_masked_ttbr(
            tramp, 0, crate::paging::TASK_STACK_VA + 4096, ttbr,
        );
    }
    let (taken, esr, far, _, _) = crate::exception::el0_capture_result();
    // Capture returns to EL1 without clearing ACTIVE; do it here.
    crate::el0::clear_active();
    crate::asidgen::release_cpu();
    crate::paging::switch_ttbr0_no_tlbi(crate::paging::kernel_ttbr0());
    crate::frame::free(pa);
    let ec = (esr >> 26) & 0x3f;
    let dfsc = esr & 0x3f;
    let fault = taken && ec == 0x24 && (0x0c..=0x0f).contains(&dfsc) && (far & !0xfff) == text_va;
    let leak = !taken;
    if cfg!(feature = "xo-leak-probe") {
        if leak {
            line(format_args!("smp: s5 xo leak el0-read-text va={text_va:#x} cpu={}", crate::percpu::id()));
        }
        leak
    } else if fault {
        true
    } else {
        line(format_args!("smp: s5 xo missed taken={taken} esr={esr:#x} far={far:#x}"));
        false
    }
}

fn s5_force_rollover_while(cpu1_asid: u64) -> bool {
    // Keep allocating until we pass at least one rollover past the current
    // generation and, under the drop-reserve negative, until cpu1_asid is
    // seen again.
    let start = crate::asidgen::rollovers();
    let mut reissued = false;
    let mut n = 0u64;
    while n < 4 * crate::asidgen::PER_GEN {
        let s = crate::asidgen::alloc();
        n += 1;
        if s.asid == cpu1_asid {
            reissued = true;
            break;
        }
        if crate::asidgen::rollovers() > start + 1 && !cfg!(feature = "asid-reserve-drop-probe") {
            // Positive path: one full generation past is enough; the reserved
            // rule must have kept cpu1_asid out of the allocator.
            break;
        }
    }
    S5_REISSUED.store(if reissued { 1 } else { 0 }, Ordering::SeqCst);
    S5_ROLLOVER_DONE.store(1, Ordering::SeqCst);
    crate::asidgen::rollovers() > start
}

/// Core 1's S5 work: hold one task's ASID across the rollover, then keep
/// draining the queue.
fn cpu1_s5() {
    let freq = cntfrq().max(1);
    while S5_READY.load(Ordering::SeqCst) == 0 {
        core::hint::spin_loop();
    }
    // First pick: hold across the rollover.
    let Some(id) = crate::runq::pick() else {
        line(format_args!("smp: s5 cpu1 missed (empty queue)"));
        S5_RESULT.store(0xff, Ordering::SeqCst);
        return;
    };
    let tasks = unsafe { &mut *core::ptr::addr_of_mut!(S5_TASKS) };
    let Some(t) = tasks.iter_mut().find(|t| t.as_ref().map(|x| x.id) == Some(id)).and_then(|t| t.as_mut()) else {
        crate::runq::put(id, true);
        return;
    };
    let asid = crate::asidgen::switch_to(&mut t.slot);
    S5_CPU1_ASID.store(asid, Ordering::SeqCst);
    S5_CPU1_GEN.store(t.slot.gen, Ordering::SeqCst);
    S5_CPU1_HOLD.store(1, Ordering::SeqCst);
    // Wait for core 0's rollover while still holding the ASID.
    let t0 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t0) < freq * 4 {
        if S5_ROLLOVER_DONE.load(Ordering::SeqCst) != 0 {
            break;
        }
        core::hint::spin_loop();
    }
    // Still the same ASID under the reserved rule.
    let still = crate::asidgen::active_on(1) == asid;
    crate::asidgen::release_cpu();
    let ok = s5_run_once(t);
    crate::runq::put(id, true);
    if ok {
        S5_CPU1_RUNS.fetch_add(1, Ordering::SeqCst);
    }
    line(format_args!(
        "smp: s5 cpu1 hold asid={asid} still-active={} run={} cpu=1",
        if still { "yes" } else { "no" },
        if ok { "ok" } else { "missed" }
    ));
    // Drain remaining tasks.
    while let Some(id) = crate::runq::pick() {
        if let Some(t) = tasks.iter_mut().find(|t| t.as_ref().map(|x| x.id) == Some(id)).and_then(|t| t.as_mut()) {
            if s5_run_once(t) {
                S5_CPU1_RUNS.fetch_add(1, Ordering::SeqCst);
            }
        }
        crate::runq::put(id, true);
    }
    let _ = still;
    // ADR-105: prove the nested BRK lands on this core's own fatal stack.
    // Noreturn on core 1; core 0 waits on CPU1_FATAL_SEEN.
    crate::exception::trigger_fatal_nested();
}

/// ADR-105 (S5) on core 0, after S4. Spawns two tasks, signals core 1,
/// runs concurrently, forces a rollover mid-hold, then checks reach/t2t/xo.
pub fn observe_s5() -> bool {
    let freq = cntfrq().max(1);
    crate::runq::reset();
    S5_READY.store(0, Ordering::SeqCst);
    S5_CPU0_RUNS.store(0, Ordering::SeqCst);
    S5_CPU1_RUNS.store(0, Ordering::SeqCst);
    S5_CPU1_ASID.store(0, Ordering::SeqCst);
    S5_CPU1_GEN.store(0, Ordering::SeqCst);
    S5_CPU1_HOLD.store(0, Ordering::SeqCst);
    S5_ROLLOVER_DONE.store(0, Ordering::SeqCst);
    S5_REISSUED.store(0, Ordering::SeqCst);
    S5_RESULT.store(0, Ordering::SeqCst);
    let a = match make_task(0, S5_MAGIC_A) {
        Some(t) => t,
        None => {
            line(format_args!("smp: s5 missed (task a)"));
            return false;
        }
    };
    let b = match make_task(1, S5_MAGIC_B) {
        Some(t) => t,
        None => {
            crate::paging::task_space_free(&a.space);
            line(format_args!("smp: s5 missed (task b)"));
            return false;
        }
    };
    let spaces_ok = s5_check_spaces(&a, &b);
    let guards_ok = s5_check_guards();
    unsafe {
        S5_TASKS[0] = Some(a);
        S5_TASKS[1] = Some(b);
    }
    let b = unsafe { &*core::ptr::addr_of!(BLOCK) };
    b.s5_go.store(1, Ordering::SeqCst);
    line(format_args!("smp: s5 go cpu0+cpu1"));
    S5_READY.store(1, Ordering::SeqCst);
    unsafe { asm!("dsb ish", "sev", options(nostack)) };

    // Wait until core 1 is holding its ASID.
    let t0 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t0) < freq * 4 {
        if S5_CPU1_HOLD.load(Ordering::SeqCst) != 0 {
            break;
        }
        core::hint::spin_loop();
    }
    let cpu1_asid = S5_CPU1_ASID.load(Ordering::SeqCst);
    if cpu1_asid == 0 {
        line(format_args!("smp: s5 missed (cpu1 never held)"));
        return false;
    }
    let rolled = s5_force_rollover_while(cpu1_asid);
    let reissued = S5_REISSUED.load(Ordering::SeqCst) != 0;

    // Drain remaining tasks on core 0.
    let tasks = unsafe { &mut *core::ptr::addr_of_mut!(S5_TASKS) };
    while let Some(id) = crate::runq::pick() {
        if let Some(t) = tasks.iter_mut().find(|t| t.as_ref().map(|x| x.id) == Some(id)).and_then(|t| t.as_mut()) {
            if s5_run_once(t) {
                S5_CPU0_RUNS.fetch_add(1, Ordering::SeqCst);
            }
        }
        crate::runq::put(id, true);
    }
    // Wait for core 1 to finish.
    let t1 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t1) < freq * 4 {
        if S5_CPU1_RUNS.load(Ordering::SeqCst) + S5_CPU0_RUNS.load(Ordering::SeqCst) >= 2 {
            break;
        }
        core::hint::spin_loop();
    }

    let r0 = S5_CPU0_RUNS.load(Ordering::SeqCst);
    let r1 = S5_CPU1_RUNS.load(Ordering::SeqCst);
    let viol = crate::runq::violations();
    let mut xo_ok = true;
    for t in tasks.iter_mut() {
        if let Some(t) = t.as_mut() {
            if !s5_xo_live(t) {
                xo_ok = false;
            }
        }
    }
    let concurrent = r0 >= 1 && r1 >= 1;
    let reserve_ok = if cfg!(feature = "asid-reserve-drop-probe") {
        if reissued {
            line(format_args!(
                "smp: s5 asid reissued while running cpu=1 asid={cpu1_asid}"
            ));
        }
        reissued
    } else {
        !reissued && rolled
    };
    // Core 1's fatal nested probe: it triggers after its hold, before park.
    let t2 = crate::timer::cntpct();
    while crate::timer::cntpct().wrapping_sub(t2) < freq {
        if CPU1_FATAL_SEEN.load(Ordering::SeqCst) != 0 {
            break;
        }
        core::hint::spin_loop();
    }
    let fatal_ok = CPU1_FATAL_SEEN.load(Ordering::SeqCst) == 1;
    if fatal_ok {
        line(format_args!("smp: s5 cpu1 fatal nested stack=own cpu=1"));
    } else {
        line(format_args!("smp: s5 cpu1 fatal missed seen={}", CPU1_FATAL_SEEN.load(Ordering::SeqCst)));
    }
    let ok = spaces_ok && guards_ok && concurrent && reserve_ok && xo_ok && viol == 0 && fatal_ok && r0 + r1 >= 2;
    line(format_args!(
        "smp: s5 runs cpu0={r0} cpu1={r1} rollover={} reissued={} reach={} xo={} viol={viol}",
        if rolled { "yes" } else { "no" },
        if reissued { "yes" } else { "no" },
        if spaces_ok { "ok" } else { "missed" },
        if xo_ok { "ok" } else { "missed" },
    ));
    if ok && !cfg!(feature = "asid-reserve-drop-probe") {
        line(format_args!(
            "smp: ok cpu0+cpu1 s5 runq=locked asid=reserved concurrent=el0 reach,t2t,xo"
        ));
    } else if cfg!(feature = "asid-reserve-drop-probe") && reissued {
        // Negative path: the miss line above is the catch; do not print ok.
    } else {
        line(format_args!("smp: s5 missed"));
    }
    // Free task frames.
    for t in tasks.iter_mut() {
        if let Some(t) = t.take() {
            crate::paging::task_space_free(&t.space);
        }
    }
    ok && !cfg!(feature = "asid-reserve-drop-probe")
}

/// Core 1 entry into S5 (called from `cpu1_kernel_main` after S4 probes).
fn cpu1_s5_entry() {
    cpu1_s5();
}
