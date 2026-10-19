# ADR-105 — SMP slice S5: concurrent EL0 on both cores, run-queue lock, reserved ASID

- Status: **Accepted for implementation** (sponsor go for S5, relayed by DSO on 2026-10-19 at 21:26 CEST). Changes **no claim**.
- Date: 2026-10-19
- Follows: [ADR-100](ADR-100-smp-race-story-plan.md) (plan, R3/R4/R5/R6), [ADR-104](ADR-104-smp-s4-percpu-asid-generation-el0-cpu1.md) (S4). Implements the "Proposed S5" section of ADR-104.
- Base: branch `smp/s4-asid-percpu-el0` (PR #160 / ADR-104), or `main` once #160 has merged.
- `_start` untouched. The default single-core build keeps its behaviour and its full smoke.
- $0: the Linux box plus the existing GitHub Actions checks. No cloud agent.

**ADR-091 revision 4 is unchanged** (one core). A separate docs-only PR proposes a multi-core claim sentence; it stays Proposed until the sponsor writes an accept.

## Decision

### 1. Run-queue lock and per-core current task (R6)

New `src/runq.rs` (`smp2-probe` only): a spin-locked table of up to 8 tasks. `pick` returns the next runnable unowned task, marks this core as its owner and stores `id+1` in `PerCpu::current`. `put` returns it (runnable or finished); an owner mismatch increments a violation counter. Round-robin, no preemption: each trip is one EL0 entry to `SYS_EXIT`.

### 2. Per-task address spaces

New `paging::TaskSpace` (`smp2-probe` only): three fresh frames (L1→L2→L3) mapping only the task's own pages in the window — text (EL0 execute-only), data (EL0 RW), stack (EL0 RW), all nG. No kernel mapping in TTBR0: vectors, exception stacks and `TPIDR_EL1`→PerCpu are TTBR1. Writes take `TABLES`. `task_space_walk` collects every EL0-reachable leaf for the reach / t2t / xo check.

### 3. Remaining shared flags banked; per-core fatal stack; guard pages

- Banked (on top of S4's 71): the run-local SVC result flags in `syscall.rs` (18), the live PAN / xo / reach flags in `pan.rs`, `xo.rs`, `reach.rs` (3+2+3). Boot-published IDs (`PAN_ID`, …) stay global.
- Core 1 gets its own fatal stack (8 KiB + 4 KiB guard). `fatal_enter` reads `PerCpu::fatal_top` (offset 24); 0 = the linker fatal stack (core 0 / default). On a secondary the fatal handler only records the stack check (no raw UART) so it cannot tear core 0's locked lines.
- Core 1's thread / exception / fatal stacks each have a 4 KiB guard page, punched at boot with the linker-stack guards (before the TTBR1 RAM clone), so the high aliases have the same holes.

### 4. Concurrent EL0 + mid-run ASID rollover (`smp2-probe`, after S4)

1. Core 0 builds two tasks (A, B), each with its own `TaskSpace` and an allocator ASID, checks reach/t2t/xo on both spaces and that core 1's guard pages are unmapped, then signals.
2. Both cores `pick` from the run queue and drive EL0 (`s5_run_once`: switch ASID, `ERET`, wait for `SYS_EXIT`, check the data magic).
3. Core 1's first pick **holds** its ASID (still marked running) while core 0 forces at least one ASID-generation rollover. The reserved-running-ASID rule (ADR-104) must keep core 1's ASID out of the allocator. Core 1 then finishes its run and drains; core 0 drains the rest.
4. Live xo: each task's EL0 load of its own text must fault (permission L3).
5. Core 1 triggers the nested-BRK fatal probe; it must land on its own fatal stack.
6. Pass line: `smp: ok cpu0+cpu1 s5 runq=locked asid=reserved concurrent=el0 reach,t2t,xo`.

### 5. Negative build `asid-reserve-drop-probe`

Same as `smp2-probe` but the ASID rollover no longer keeps running ASIDs reserved (and `switch_to` will not keep an older generation's running ASID). The probe must print `smp: s5 asid reissued while running cpu=1 asid=…` and must not print the S5 ok line; the checker must fail.

### 6. Smoke / CI

- `smp2_check` requires the S5 lines. New S5 negative ELF in the opt-in smoke.
- Workflow comment on the `qemu-smp2` job updated (ADR-100..105; EL0 on the secondary is no longer denied). Job stays not required. Timeout stays 30 minutes.

## TCG honesty

- Same ASID-tagging limit as ADR-104: QEMU flushes a vCPU's soft TLB on an ASID change. The reserved-ASID proof is that the allocator did not reissue core 1's ASID during the hold (positive) / did reissue it (negative), not that a stale TLB entry survived a switch.
- Concurrent EL0 on TCG is real (both vCPUs enter EL0 and take SVCs); scheduling quanta are TCG's, not a wall-clock claim.
- Observed on the Linux box, QEMU 10.0.13 TCG, `-cpu cortex-a76`, `-smp 2`, 2026-10-19: the S5 positive build printed `smp: ok cpu0+cpu1 s5 …` on **10 of 10** runs; `asid-reserve-drop-probe` printed `smp: s5 asid reissued while running cpu=1` on **10 of 10** runs (no S5 ok line). Full `CTOS_SMP2_SMOKE=1` smoke: `qemu-smoke: ok` (default smoke, single-core leaks, S1–S5 positive and all negatives).

## What this proves

- A locked run queue hands each task to one core at a time; owner mismatches stay at 0.
- Two tasks with private tables and allocator ASIDs run EL0 on both cores in one session; each space has exactly the three expected EL0 leaves (xo text, RW data, RW stack) and neither data frame appears in the other's walk; EL0 load of own text faults.
- A mid-run ASID rollover keeps core 1's running ASID reserved; dropping that rule reissues it and is caught.
- Core 1's fatal nested BRK runs on its own fatal stack; its three guard pages are unmapped (identity and high).

## What this does not prove

- Not a general-purpose SMP OS. No preemption, no IRQ-driven reschedule, no multi-priority queue, no user-facing API.
- The default build still uses fixed ASIDs 1 and 2; the allocator and run queue are opt-in only.
- Boot-published result flags, the shared user / ASID-B tables used by the single-core probes, virtio DMA rings and VFS locks are unchanged. The S5 tasks do not share the single-core EL0 probe page.
- Nothing about real hardware, KVM or other QEMU versions. ADR-091 rev 4 stays one core.

## Proposed next (not started)

- Wire the allocator into the single-core loader path (retire fixed ASIDs 1/2 for real tasks).
- IRQ-driven preemption across cores; per-core timer ticks counted separately.
- Sponsor accept of the multi-core claim sentence (separate docs-only PR).

## Consequences

- Default kernel: core 1's stacks add ~96 KiB to `.bss` in the opt-in build only (the `Guarded` statics are behind `smp2-probe`… actually they are in smp.rs which is cfg'd). Default build unchanged.
- Opt-in CI job builds one more ELF (S5 reserve-drop negative). Still not required.
