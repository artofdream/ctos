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

It does not time QEMU boot.

## Boot-to-ready CNTPCT (NFR-08, this tree)

`kernel_main` samples `CNTPCT_EL0` after `paging::init` (MMU + D-cache on) and again after `Hello World!` (init complete):

- Serial marker `perf: boot-delta ticks=<n>` (fail closed on `perf: boot-delta missed`).
- `#[test_case]` asserts a sample exists and the counter advanced.

That is **kernel_main-entry to after-init** on this QEMU virt guest. It is not a latency budget, not QEMU startup time, not a published bench, and not criterion.

## OS/app slot disconnect (A9) — compared pair (QEMU TCG lab)

[A9 #48](https://github.com/artofdream/ctos/issues/48) / [ADR-030](../03-adr/ADR-030-os-app-slots.md) loads a **separate app payload** from FAT `/hello` after the OS image ([immutability.md](immutability.md), site [measure.md](../overview/measure.md)). The load path prints `perf: app-load ticks=<n>`. [ADR-046](../03-adr/ADR-046-slot-perf-delta.md) adds a **probe-only** linked-in trip of the same `hello-libctos.elf` bytes (`include_bytes!` in `src/slot.rs` only) that prints `perf: embed-load ticks=<n>`, then an honest pair `perf: slot-delta app=<a> embed=<b>`. Production A9 stays FAT-only (`slot: embed` must never print). That is a **measurement pair**, not a bench and not a percent.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **FAT path** | VFS/FAT read + ELF parse + user map + `ERET` | `perf: app-load ticks=<n>` |
| **Probe-only embed** | Same bytes already in the kernel image; parse + map + `ERET` (no FAT) | `perf: embed-load ticks=<n>` — not `slot: embed` |
| **Pair** | Raw tick counts on one boot | `perf: slot-delta app=<a> embed=<b>`. No “faster/slower.” No invented percent. QEMU TCG jitter. |
| **Operational** | Smaller OS updates without rebuilding apps | Operational ≠ measured latency. A2–A4 stay FAT-only ([ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). |
| **Gate** | Keep `perf: boot-delta`. Fail closed on missing `perf: app-load` / `perf: embed-load` / `perf: slot-delta` | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. No `criterion` crate. No “slot disconnect is free.”


## FAT16 vs memfs write (ADR-051) — compared pair (QEMU TCG lab)

[ADR-050](../03-adr/ADR-050-fat16-write.md) writes a small payload through the thin VFS onto the FAT16 volume. [ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md) samples `CNTPCT` around that **single** `vfs::write` (`perf: fat-write ticks=<n>`), then on the same boot times a memfs `vfs::write` of the **same** bytes (`perf: memfs-write ticks=<n>`), and prints `perf: fs-write-delta fat=<a> memfs=<b>`. That is a **measurement pair**, not a bench and not a percent.

| Class | What we measure | Honesty |
| --- | --- | --- |
| **FAT write** | One VFS write of the `/probe` rewrite payload (`fat-wr`) | `perf: fat-write ticks=<n>` |
| **Memfs write** | One VFS write of the same bytes on `/mwprobe` | `perf: memfs-write ticks=<n>` |
| **Pair** | Raw tick counts on one boot | `perf: fs-write-delta fat=<a> memfs=<b>`. No “faster/slower.” No invented percent. QEMU TCG jitter. |
| **Gate** | Keep `fat: write` / `fat: ok` / A9 markers. Fail closed on missing pair markers | Verified only when the ledger has serial evidence on a named tip. |

QEMU TCG jitter is still one lab. No `criterion` crate. No latency SLA for FAT write.

## Later probes (Planned)

- A tighter “first instruction of `_start`” sample if someone maps a `.data` slot that BSS-clear will not wipe.

Do not add a host `criterion` crate or a “bench.yml” that prints invented numbers.
