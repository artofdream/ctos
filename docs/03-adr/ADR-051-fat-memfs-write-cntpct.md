# ADR-051 — FAT16 vs memfs write CNTPCT pair (measure-first)

- Status: Accepted (compared CNTPCT pair on one boot — QEMU TCG lab measurement, not a published bench).
- Date: 2026-09-13

## Context

[ADR-050](ADR-050-fat16-write.md) landed guest FAT16 write behind the thin VFS (`fat: write` / `fat: rewrite` / `fat: create` / `fat: ok`). Existing performance probes cover loop CNTPCT, IRQ delta, boot-delta, ELF size, and the A9 slot pair ([ADR-046](ADR-046-slot-perf-delta.md)). Future slice 4 asks for a **measure-first** CNTPCT sample around the new FAT write path, compared on the **same boot** to a memfs write of the same small payload.

Do **not** invent a latency SLA, percent, or “faster/slower” claim. Do **not** add a `criterion` crate. Keep `fat: write` / `fat: ok` / A9 slot markers. QEMU TCG jitter is one lab — not SPEC.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Time the FAT VFS `write` of the `/probe` rewrite payload (`fat-wr`) with CNTPCT → `perf: fat-write ticks=<n>`. On the same boot, time a memfs `write` of the same bytes → `perf: memfs-write ticks=<n>`. Print `perf: fs-write-delta fat=<a> memfs=<b>` (raw ticks). | Honest compared pair. Not a bench. |
| **H2 (rejected)** | Publish a percent / “FAT is slower than memfs” / criterion bench. | NFR-07 forbids invented benches. |
| **H3 (rejected)** | Time the whole open/read/restore/create path as one number and call it “write latency.” | Too wide; pair the write of the same payload. |
| **H4 (rejected)** | Skip the memfs compare and only print FAT ticks. | Prefer a same-boot pair when feasible (same shape as ADR-046). |

## Decision

1. **FAT write sample.** During the ADR-050 `/probe` rewrite, sample `CNTPCT` around the single `vfs::write` of `WRITE_BYTES` (`fat-wr`). Store ticks; fail closed if the counter does not advance.
2. **Memfs compare.** After the FAT write/create probes succeed, `vfs::create`/`open` a memfs path (`/mwprobe`) and sample `CNTPCT` around `vfs::write` of the **same** bytes. Fail closed if ticks are zero.
3. **Markers.** Serial: `perf: fat-write ticks=<n>`, `perf: memfs-write ticks=<n>`, `perf: fs-write-delta fat=<a> memfs=<b>`. No ratio percent; no “faster/slower” words on the serial line. Missed samples print `perf: fat-write missed` / `perf: memfs-write missed`.
4. **Existing markers unchanged.** Keep `fat: write` / `fat: rewrite` / `fat: create` / `fat: ok` and A9 `slot:*` / ADR-046 pair.
5. **Smoke.** `scripts/qemu-smoke.sh` greps the new markers and rejects `fs-write-delta` lines that claim percent/faster/slower.
6. **Honesty.** This is a **measurement pair** on QEMU `virt` TCG. It is not a latency budget, not a product KPI, and not “FAT write cost is X.” Record Verified only with serial evidence on a named tip. Do not claim app hosting done, EL0 isolated, POSIX, or FAT32.

## Consequences

- Code: `src/fat.rs` (CNTPCT around FAT write + memfs compare + markers); smoke greps; `#[test_case]` that both samples are `> 0`.
- Docs: this ADR, [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, roadmap `P-PERF-6`.
- Follow-ups (not this PR): multi-cluster write cost, create-path timing, host-side FS benches — only after a probe shows a need.
