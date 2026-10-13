# ADR-100 — The SMP race story: plan, global-state inventory, and S1 secondary bring-up smoke

- Status: **Proposed** (plan) + **S1 implemented as an opt-in smoke** in the same PR. The plan does not change any claim.
- Date: 2026-10-13
- Decision path: milestone item 5 ("the SMP race story"), sponsor approval 2026-10-13, relayed by DSO. $0 work: one local machine plus the existing GitHub Actions checks.
- Base: `main` at `af5a9d1` (ADR-091 rev 4 recorded, PR #153).
- `_start` (`0x4008_0000`) untouched. The secondary entry is a separate symbol, `_secondary_entry`, in the same boot-stub page.

## Context

ctos runs on one core. The accepted claim, [ADR-091](ADR-091-el0-isolated-accept-draft.md) revision 4, scopes "EL0 isolated" to `-cpu cortex-a76`, TCG, **one core**. **That sentence does not change in this ADR or in any PR of this story.** A multi-core claim of any kind needs a new, separate written sponsor accept.

Every piece of kernel code was written on the one-core assumption: "nothing else runs while I hold this". On two cores that is false. This ADR writes down, before any code relies on it, where the kernel keeps global mutable state, what each piece assumes, what goes wrong on a second core, and the order in which the gaps get closed, each with a probe that fails closed.

## Inventory of global mutable kernel state (main `af5a9d1`)

Method: `rg` for `static` / `static mut` items under `src/` (excluding the new `src/smp.rs`), then reading each owner module. **262** static items:

| Kind | Count | Where |
| --- | --- | --- |
| `static mut` (no lock at all) | **25** | `paging.rs` 19 (page-table storage `L1`/`L2`/`L3`/`L2_RAM`/`L3_KERNEL[]`, `*_USER`, `*_ASID_B`, `*_HIGH`, `*_HIGH_RAM`, and three "used" bump counters); `virtio.rs` 3 (`DMA`, `NET_RX`, `NET_TX`); `el0.rs`, `ro.rs`, `teardown.rs` 1 each (bait / magic words) |
| `spin::Mutex`-wrapped | **10** | `UART`, heap `ALLOCATOR`, frame `ALLOC`, `SCHED`, VFS `FS` + `LAST_READ`, `FAT`, virtio `DEV` + `NET`, and `paging::TABLES: Mutex<()>` (a convention lock: 44 `TABLES.lock()` sites guard the `static mut` tables, but the type system does not force it) |
| Atomic flags / counters | **208** | mostly probe bookkeeping (`EXPECT_*` / `*_CAUGHT` in `exception.rs` 61, `paging.rs`, `syscall.rs`, `el0.rs`, `pan.rs`, `sched.rs`, `timer.rs`) |
| Immutable / linker symbols | 19 | `__stack_top` etc., embedded ELF, magic constants |

Plus **per-CPU hardware state that the kernel treats as global**: one boot stack (`__stack_top`), one exception stack, one fatal stack (linker symbols), `VBAR_EL1`, `TTBR0/1_EL1`, `TCR`, `SCTLR` (incl. `SPAN`), `PSTATE.PAN`, `TPIDR_EL1` (holds the saved kernel TTBR0 for the EL0 trampoline), `DAIF`, the generic timer, the GIC CPU interface.

### What each piece assumes and how it races on two cores

| # | State | One-core assumption | Race on two cores | Risk |
| --- | --- | --- | --- | --- |
| R1 | Page-table storage + bump counters (`static mut`, `paging.rs`) | `TABLES` is taken by convention (44 sites); not audited that every writer (early boot, tears, relocations, leak-probe plants) holds it; `L3_*_USED = n + 1` is a plain read-modify-write | Any writer outside `TABLES` races a builder on the other core: same L3 table handed out twice, lost or crossed mappings, an EL0-visible leaf in the wrong root | **High** (isolation) |
| R2 | TLB maintenance (`tlbi vmalle1`, `tlbi vaae1`, no `IS` suffix anywhere; `switch_ttbr0_no_tlbi`) | Invalidating the local TLB is enough | A torn / unmapped page (identity tears, `unmap`, ASID-B private page) stays cached in the other core's TLB; EL0 or EL1 on that core still reaches it | **High** (isolation; every "torn" claim) |
| R3 | ASIDs are fixed constants (1 = user root, 2 = ASID-B), no allocator, no generation | Only one task holds an ASID at a time on one TLB | Two cores running different roots under one ASID share TLB tags (stale translations across tasks); ADR-096 t2t "no-tlbi" argument holds per core only | **High** |
| R4 | `TPIDR_EL1` = saved kernel TTBR0; one exception / fatal stack | One CPU, one nesting chain | Each core needs its own; sharing one exception stack is memory corruption on the first simultaneous exception | **High** (correctness) |
| R5 | `el0.rs` `ACTIVE` / `USER_PC` / `USER_SP` / `USER_TTBR` / `KIND`; `exception.rs` `EXPECT_*` / `*_CAUGHT` | "The" EL0 context and "the" expected fault | A fault on core B is accepted as core A's expected probe fault (false pass) or the restore path resumes the wrong context | **High** (probe integrity) |
| R6 | `SCHED` (`spin::Mutex`) + `TASK_A_SP` / `TASK_B_SP`; unlock before `context_switch` | Lock is only against re-entry; one runner | Both cores pick and run the same task on one stack | High |
| R7 | Heap `ALLOCATOR`, frame `ALLOC` (`spin::Mutex`) | No IRQ path allocates; contention never happens | Mutual exclusion holds across cores (spin locks are SMP-safe), but not IRQ-safe: a lock taken in IRQ context on the same core deadlocks; frame free/alloc vs. page-table use needs TLB ordering (R2) | Medium |
| R8 | virtio `DMA` / `NET_RX` / `NET_TX` (`static mut`) and the descriptor rings | Single driver user; polled completion | Two cores post to one ring; double-used descriptors, corrupted DMA buffers | Medium (no isolation claim depends on it) |
| R9 | VFS `FS`, `FAT`, `LAST_READ` (`spin::Mutex`) | Whole-op serialisation by one caller | Lock-correct per call, but multi-call sequences (open → read → close, mkdir) are not atomic | Low/Medium |
| R10 | `UART` (`spin::Mutex`) + `write_str_raw` / `raw()` (no lock, used by fatal/exception paths and probes) | Raw writes never interleave | Interleaved bytes from two cores break line-based markers; a fatal path on one core while the other holds `UART` | Low for safety, **High for evidence** (smoke grep) |
| R11 | GIC distributor / timer init, `timer` tick counters | One CPU interface, one timer | Secondary needs its own GICC + timer init; SPI routing; tick counts are per-core | Medium |
| R12 | `PSTATE.PAN`, `SCTLR.SPAN`, `TCR`, `VBAR` | Set once at boot on the one core | A secondary that does not repeat ADR-080/098 setup runs with PAN off / wrong vectors | **High** (the ADR-091 PAN clause) |

**Riskiest:** R2 (local-only TLBI), R1 (unlocked page-table builders), R3 (fixed ASIDs), R5 (global "expected fault" flags), R12 (per-core PAN/SPAN/VBAR setup). Any of these alone would make a multi-core "EL0 isolated" claim false.

## Plan

### Locking / ownership

- **Page tables (R1):** make `TABLES` the only way in: move the tables and bump counters into the `Mutex` payload (type-enforced), mask IRQs while held, audit every current `static mut` writer. No lock-free page-table writes.
- **TLB (R2):** every unmap, permission downgrade or tear that may be cached on another core uses the Inner-Shareable forms (`tlbi vae1is` / `vaae1is` / `vmalle1is` / `aside1is`) followed by `dsb ish; isb`. Local forms stay only where the mapping was never visible to another core (documented per site). No IPI-based shootdown is needed for the ARMv8 broadcast case on QEMU `virt`; an IPI path is only planned if a probe shows broadcast is insufficient.
- **ASIDs (R3):** a small allocator with a generation counter; on roll-over, `tlbi vmalle1is` and bump generation; each core checks its current task's generation on switch. Until then: secondaries never run EL0 (S1–S3).
- **Per-CPU (R4, R5, R11, R12):** a `PerCpu` block per core (stacks, saved TTBR0, EL0 context, expected-fault flags, timer state), pointed to by `TPIDR_EL1`; the trampoline's current use of `TPIDR_EL1` moves into that block. Probe "expect/caught" flags move to per-CPU.
- **Scheduler (R6):** one run-queue lock, task state `Running(cpu)`; a task is never picked while running elsewhere.
- **Heap / frames (R7):** keep spin mutexes; mask IRQs while held; forbid allocation in IRQ handlers (documented check).
- **Drivers / FS (R8, R9):** stay owned by CPU 0 (other cores reach them only via a syscall that is serviced on CPU 0) until a probe needs more.
- **UART (R10):** one console lock that also covers `raw()` writes in non-fatal paths; per-line atomic writes; fatal path may break the lock (documented).

### Secondary bring-up

PSCI `CPU_ON` (function `0xC400_0003`) via `HVC #0` — QEMU `virt` with `-kernel` and no EL2/EL3 firmware exposes PSCI on the HVC conduit, with secondaries powered off. Entry is a physical address; the secondary starts at EL1 **MMU off**. Later slices give it its own stack, `VBAR`, `TCR`/`MAIR`, `TTBR1` (shared kernel high map), `SCTLR` (incl. `SPAN=0`), PAN set, GICC + timer, then a park/run loop.

### Isolation probes that must be re-proved per core (S4)

On each core: ADR-080/098 PAN enabled + held; ADR-013/039 EL0 cannot read/execute kernel; ADR-092 EL0-reach walk (shared tables, but walk from each core's live `TTBR0`); ADR-094 syscall pointer checks; ADR-096 task-to-task; ADR-097 execute-only text; ADR-087/088 identity inventory (each core's TTBR0); plus a **new cross-core TLB probe**: unmap on core 0, access on core 1 must fault (negative build: local-only TLBI must be caught).

### PR sequence (each with a fail-closed acceptance probe)

| Slice | Content | Acceptance probe (fail-closed) | Negative test |
| --- | --- | --- | --- |
| **S1 (this PR)** | Plan + opt-in `smp2-probe`: CPU_ON one secondary, MMU off, print MPIDR, store it to a flag, park | `smp: secondary up mpidr=0x0000000080000001` and `smp: ok cpu1 … parked` under `-smp 2`; default smoke must show **no** `smp:` line | Same ELF under `-smp 1` → `rc=-2`, `smp: cpu1 missed`; checker must fail |
| S2 | Secondary MMU on: shared TTBR1, own stack/VBAR/SPAN=0/PAN; PerCpu via `TPIDR_EL1`; console lock | `smp: cpu1 mmu-on pan=1 span=0 vbar=high` | build that skips SPAN clear on cpu1 must print `smp: cpu1 pan lost` |
| S3 | Type-enforced `TABLES` + Inner-Shareable TLBI everywhere + cross-core unmap probe | `smp: xtlb unmap cpu0 → cpu1 fault` | `local-tlbi-leak-probe` build (old local TLBI) must report `smp: xtlb leak` |
| S4 | ASID allocator with generation; per-CPU EL0 context + expect flags; re-run each isolation probe on cpu1 (still kernel-driven, one EL0 task per core) | each probe's existing `… ok` marker tagged `cpu=1` | existing leak builds run on cpu1 must still be caught |
| S5 | Scheduler run-queue lock, EL0 tasks on both cores; docs-only proposal of a multi-core sentence for the sponsor | all of the above in one boot, under `-smp 2` CI job | — |

The `-smp 2` CI job stays **non-required** until the sponsor accepts a multi-core claim.

## What will NOT be claimed

- Not "SMP support", not "multi-core safe", not "EL0 isolated on multiple cores". ADR-091 rev 4 stays one core.
- S1 does not run EL0, the scheduler, the heap, page-table code, drivers or interrupts on the secondary. The secondary runs MMU off, prints, writes one word, parks.
- No claim about real hardware, KVM, other `-cpu` models, more than two cores, or PSCI firmware other than QEMU's built-in HVC conduit.
- No memory-ordering / data-race freedom claim for any existing module. The inventory is by code reading; it may miss state reached through raw pointers.
- No performance or scaling claim.

## S1 decision (this PR)

- Cargo feature `smp2-probe` (opt-in, default off). After the identity tears and before GIC/virtio (same spot as `b2-serror`), the primary prints `smp: cpu0 mpidr=…`, cleans the flag line, calls PSCI `CPU_ON(target=1, entry=PA(_secondary_entry), context=PA(flag))`, waits up to 0.5 s polling the flag (with `dc civac`), then prints `smp: cpu_on target=0x1 rc=…` and `smp: ok cpu1 mpidr=… parked` or `smp: cpu1 missed seen=…`, then parks.
- `_secondary_entry` (in `.text.boot`, after `_start`, PA `0x4008_0048` on this build): MMU off, raw PL011 writes of `smp: secondary up mpidr=0x<16 hex>`, store MPIDR to the context PA, `dsb sy; sev`, `wfe` loop. No stack, no Rust.
- `scripts/qemu-smoke.sh`: `smp2_check` + `smp2_smoke`. `CTOS_SMP2_SMOKE=1` adds it after the default run; `CTOS_SMP2_ONLY=1` runs only the id guard + this. Positive under `-smp 2`, negative under `-smp 1` (checker must fail and `smp: cpu1 missed seen=0x0` must appear). The default run gains a check that **no** `smp:` line appears.
- CI: a new **non-required** job `QEMU -smp 2 bring-up (opt-in, ADR-100; not required)` runs `CTOS_SMP2_ONLY=1`. The three required checks are unchanged.

## Consequences

- Default kernel binary: no code from `src/smp.rs` (module is `#[cfg(feature = "smp2-probe")]`). The default smoke proves it still passes and has no `smp:` marker.
- `_secondary_entry` exists only in the `smp2-probe` build (it lives in `src/smp.rs`), so the default boot-stub page is byte-identical in layout; the ADR-087 inventory on the default run still reports the one stub page (`k=1 u=1 a=1`).
- Next: S2.
