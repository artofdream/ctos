# Performance (NFR-07 / ADR-011)

Performance is a first-class pillar, not a Later learning-only note. That does **not** authorize fake benches.

## Rules

1. A latency, cycle, or “faster than” sentence is a **claim**. It needs a probe in the [honesty ledger](honesty-ledger.md).
2. Optimize only after a probe shows a cost. Do not rewrite the scheduler or heap “for speed” on a hunch.
3. QEMU `virt` numbers are one environment. They are not a Raspberry Pi probe and not a published SPEC run.

## Baseline probe (this tree)

`CNTPCT_EL0` is already used to timeout the M5 timer observe window. The pillars ratchet (when landed) measures a **fixed trivial loop**:

- Serial marker `perf: cntpct delta=<n>` (fail closed on `perf: probe missed`).
- `#[test_case]` asserts the counter advanced.

That probe proves the physical counter is readable and moves. It does **not** claim a microsecond budget, interrupt latency, or a comparison to other kernels.

## Later probes (Planned)

- Timer-tick jitter (spread of CNTPCT deltas between CNTP firings).
- Debug image size / boot time (NFR-08) once someone actually measures them.

Do not add a host `criterion` crate or a “bench.yml” that prints invented numbers.
