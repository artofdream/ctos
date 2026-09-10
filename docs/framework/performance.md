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

## IRQ-to-handler probe (this tree)

When CNTP fires, the handler records `CNTPCT − CNTP_CVAL` before rearm. After several ticks the hello kernel prints:

- Serial marker `perf: irq-delta min=<a> max=<b> spread=<b-a> n=<n>` (fail closed on `perf: irq-delta missed`).
- `#[test_case]` asserts samples exist and `max >= min`.

That is a **spread of IRQ-to-handler counter deltas on this QEMU virt guest**. It is not a latency budget, not “faster than X,” and not a published bench. QEMU TCG jitter is one environment.

## Host debug ELF size (NFR-08, this tree)

`scripts/qemu-smoke.sh` prints the host byte size of `target/aarch64-ctos/debug/ctos` after `cargo build`:

- Host marker `perf: elf-size bytes=<n>` (fail closed if missing or `< 4096`).
- This is a **measurement**, not a size budget and not a “smaller is better” claim.

It does not time QEMU boot. Boot-time CNTPCT remains unprobed (still Planned if someone wants it).

## Later probes (Planned)

- Guest boot-to-ready CNTPCT (NFR-08 boot time) once someone actually measures it.

Do not add a host `criterion` crate or a “bench.yml” that prints invented numbers.
