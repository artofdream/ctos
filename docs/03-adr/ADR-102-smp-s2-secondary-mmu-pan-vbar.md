# ADR-102 — SMP slice S2: secondary core MMU on, own stack and vectors, PAN/SPAN, console lock

- Status: **Accepted for implementation as an opt-in probe** (sponsor go for S2, 2026-10-13 about 22:45 CEST, relayed by DSO). Changes **no claim**.
- Date: 2026-10-13
- Follows: [ADR-100](ADR-100-smp-race-story-plan.md) (plan + S1). Implements its slice **S2**, with the deviations listed below.
- Base: `main` at `a6cb41e` (#154 merged).
- `_start` (`0x4008_0000`) untouched. The secondary still enters at `_secondary_entry` in the boot-stub page.
- $0: one local machine plus the existing GitHub Actions checks. No cloud agent.

## Scope (from the sponsor go)

Bring core 1 up properly: MMU on with the shared kernel tables, its own stack, its own `VBAR_EL1` vectors, PAN/SPAN set the same way as core 0 (ADR-080 / ADR-098), and a console lock so the two cores do not interleave UART output. A fail-closed probe proves it; negative builds must be caught.

**ADR-091 revision 4 is unchanged:** "EL0 isolated" stays scoped to `-cpu cortex-a76`, TCG, **one core**. No SMP safety claim of any kind.

## Decision

All of it stays behind the opt-in Cargo feature `smp2-probe` (default build: no `src/smp.rs` code; the default smoke still fails on any `smp:` line).

1. **Parameter block.** Core 0 fills a 64-byte-aligned block with the values it is running on: `MAIR_EL1`, `TCR_EL1`, the live `TTBR0_EL1` (kernel root) and `TTBR1_EL1` (shared kernel high map), and `SCTLR_EL1` **with SPAN forced back to its reset value 1** (so core 1 must clear it itself). Plus the high-VA top of a dedicated 16 KiB secondary stack, the high VA of `cpu1_main`, and the block's own high VA. Core 0 cleans the block to the point of coherency (`dc civac`), because core 1 reads it with the MMU off.
2. **`_secondary_entry` (MMU off).** Unchanged S1 part: raw MPIDR line, MPIDR stored to the block's first word, `dsb sy; sev`. New: program MAIR/TCR/TTBR0/TTBR1, `tlbi vmalle1` (local; core 1's TLB has seen nothing), write SCTLR (MMU, caches, WXN on), switch `sp` to the secondary stack, branch to `cpu1_main` at the TTBR1 alias. The instruction after the SCTLR write runs from the boot-stub page, which stays identity-mapped executable in the kernel TTBR0 (the same exception `_start` already relies on).
3. **`cpu1_main` (MMU on, high alias, own stack).** Installs **its own** 2 KiB vector table `ctos_cpu1_vectors` (not core 0's, so no exception on core 1 can land on core 0's exception / fatal stacks, ADR-100 R4). Clears `SCTLR_EL1.SPAN`, sets `PSTATE.PAN`. Reads back SCTLR, `PSTATE.PAN`, `VBAR_EL1`, `sp` and `pc` and prints them.
4. **Deliberate exception.** Only if MMU, VBAR and PAN read back correctly. Core 1 clears PAN, executes `brk #0x5102`, and its own current-EL SPx sync handler (assembly, on core 1's stack) records `ESR_EL1`, `ELR_EL1`, `PSTATE.PAN` at entry and an entry count, steps `ELR` past the `brk` and `eret`s. Core 1 then sets PAN again. Pass requires: one entry, `ESR = 0xf2005102` (EC=0x3C BRK, IL=1, imm=0x5102), `ELR` = the `brk` address, **PAN=1 inside the handler** (SPAN=0 sets it on entry, the ADR-098 property), PAN=1 after. Any other vector slot records which slot fired and parks the core, so core 0 times out and reports a miss.
5. **Console lock.** After its raw MMU-off line, every `smp:` line from either core goes through the existing `uart::UART` spin lock, held for one whole line. Core 0 also stops using raw writes once core 1 may be printing (`smp: done` included). After core 1's checks, core 1 waits for a `go` word, then both cores print four `smp: tick cpuN i` lines at the same time.
6. **Verdict by core 0.** Core 0 waits up to 2 s for core 1's verdict word, re-reads what core 1's handler recorded, and prints `smp: ok cpu1 s2 mmu-on pan=1 span=0 vbar=own exc=handled console=locked` only if the verdict is OK, no stray vector fired, four ticks were counted and the recorded ESR / count match. Otherwise `smp: cpu1 s2 missed verdict=… bad=… ticks=…`.

### Negative builds (each implies `smp2-probe`; checker must fail and the miss line must appear)

| Feature | What it breaks on core 1 | Caught by |
| --- | --- | --- |
| `smp2-neg-pan` | skips `msr pan, #1` | `smp: cpu1 pan lost (pstate pan=0)`, verdict 3, no exception taken |
| `smp2-neg-span` | skips the SPAN clear | handler sees PAN=0: `smp: cpu1 pan lost (handler pan=0 span=1)`, verdict 4 |
| `smp2-neg-vbar` | installs core 0's vectors instead of its own | `smp: cpu1 vbar mismatch`, verdict 5; the exception is **not** taken (would hit core 0's stacks) |

Plus the S1 negative: the same S2 ELF under `-smp 1` (`rc=-2`, `smp: cpu1 missed seen=0x0`).

### Checker (`scripts/qemu-smoke.sh`, `smp2_check`)

Requires every S1/S2 marker line exactly, each of the eight tick lines exactly once, rejects any `smp:` line that is not one whole known line (a torn or interleaved line fails closed), and rejects any miss line. `CTOS_SMP2_ONLY=1` (the existing non-required CI job) runs: positive `-smp 2`, `-smp 1` negative, then the three negative builds. The job name and required checks are unchanged; the job stays **not required**.

## Deviations from ADR-100's S2 row

- **PerCpu via `TPIDR_EL1` is not done here.** Core 1 does not use `TPIDR_EL1` (it never enters the EL0 trampoline). Moved to S4, where the per-CPU EL0 context and expect flags need it.
- The planned marker `smp: cpu1 mmu-on pan=1 span=0 vbar=high` became the more specific `smp: cpu1 regs …` + `smp: ok cpu1 s2 …` lines; the planned single negative ("skip SPAN clear → `pan lost`") is `smp2-neg-span`, plus two more.
- S1's `smp: ok cpu1 … parked` became `smp: ok cpu1 … up` (core 1 no longer parks MMU off).

## What the probe proves (QEMU `virt`, TCG, `-cpu cortex-a76`, `-smp 2`)

- Core 1 runs with `SCTLR_EL1.M=1`, from the TTBR1 high alias, on its own stack, with the same MAIR/TCR/TTBR0/TTBR1 values as core 0 at that moment.
- Core 1's `VBAR_EL1` is its own table; one synchronous exception taken on core 1 was handled by that table on core 1 and returned.
- Core 1 has `PSTATE.PAN=1` and `SCTLR_EL1.SPAN=0`; PAN was set by hardware on exception entry. Each of the three settings, when skipped, is caught.
- With the lock, the observed output lines are whole on this run.

## What it does not prove

- **Not SMP support, not multi-core safety, not "EL0 isolated" on two cores.** Core 1 never runs EL0, the scheduler, the heap, page-table builders, drivers, the GIC or the timer.
- Nothing about TLB coherence between cores (S3), ASIDs (S4), concurrent page-table writes (R1), or that the tables core 1 sees stay valid when core 0 changes them.
- The console lock is not shown to be **necessary**: there is no deterministic negative for interleaving (it depends on host thread timing), so only the positive "lines are whole" is checked. Fatal / raw paths still bypass the lock (ADR-100 R10).
- Core 1's stack has no guard page. Core 1's vectors handle exactly one synchronous slot; IRQ/FIQ/SError on core 1 park it.
- No claim about real hardware, KVM, other `-cpu` models, more than two cores, or other PSCI firmware.

## Proposed S3 (for sponsor go; not started)

Type-enforced `TABLES` (tables and bump counters inside the `Mutex` payload, IRQs masked while held) and Inner-Shareable TLB maintenance (`tlbi …is` + `dsb ish; isb`) at every unmap / downgrade / tear site, with a cross-core probe: core 1 (MMU on, as in S2) reads a probe page; core 0 unmaps it with the broadcast form; core 1 must fault on its own vectors (`smp: xtlb unmap cpu0 → cpu1 fault`). Negative build `local-tlbi-leak-probe` uses the old local `tlbi` and must report `smp: xtlb leak` (core 1 still reads the page through its stale TLB entry). Still no EL0 on core 1; ADR-091 rev 4 unchanged.

## Consequences

- Default kernel and smoke unchanged (no `smp:` line; required checks unchanged).
- The opt-in CI job now builds four ELFs (positive + three negatives); it stays within its 30-minute timeout.
