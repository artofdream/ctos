# ADR-065 — FAT16 vs memfs read CNTPCT pair (measure-first)

- Status: Accepted (compared CNTPCT pair on one boot — QEMU TCG lab measurement, not a published bench).
- Date: 2026-09-16

## Context

[ADR-051](ADR-051-fat-memfs-write-cntpct.md) landed a same-boot FAT vs memfs **write** CNTPCT pair (`perf: fat-write` / `perf: memfs-write` / `perf: fs-write-delta`). Existing performance probes also cover loop CNTPCT, IRQ delta, boot-delta, ELF size, and the A9 slot pair ([ADR-046](ADR-046-slot-perf-delta.md)). Perf slice 3 asks for another **measure-first** compared pair around a known path. The natural symmetric cut is the small FAT `/probe` **read** versus a memfs read of the same bytes on the same boot.

Do **not** invent a latency SLA, percent, or “faster/slower” claim. Do **not** add a `criterion` crate. Keep `fat: read` / `fat: write` / `fat: ok` / A9 slot markers and the ADR-051 write pair. QEMU TCG jitter is one lab — not SPEC. Do not touch Track N / virtio-net.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Time the FAT VFS `read` of `/probe` (`fat-hi`) with CNTPCT → `perf: fat-read ticks=<n>`. On the same boot, seed memfs `/mrprobe` with the same bytes and time `vfs::read` → `perf: memfs-read ticks=<n>`. Print `perf: fs-read-delta fat=<a> memfs=<b>` (raw ticks). | Honest compared pair. Not a bench. |
| **H2 (rejected)** | Publish a percent / “FAT is slower than memfs” / criterion bench. | NFR-07 forbids invented benches. |
| **H3 (rejected)** | Time open+read+close as one number and call it “read latency.” | Too wide; pair the read of the same payload (open outside the window). |
| **H4 (rejected)** | Only print FAT ticks without a memfs compare. | Prefer a same-boot pair when feasible (same shape as ADR-046 / ADR-051). |
| **H5 (rejected)** | Time SVC yield / memcpy instead of FAT read this mile. | Useful later; FAT read is the direct follow-on to ADR-051. |

## Decision

1. **FAT read sample.** During the existing `/probe` VFS read proof, sample `CNTPCT` around the single `vfs::read` that expects `PROBE_BYTES` (`fat-hi`). Store ticks; fail closed if the counter does not advance.
2. **Memfs compare.** After FAT probes that store the FAT sample, `vfs::create`/`open` a memfs path (`/mrprobe` — added to the ADR-058 prefix table so it does not fall through to FAT `/`), write the **same** bytes outside the timed window, reopen, and sample `CNTPCT` around `vfs::read`. Fail closed if ticks are zero or bytes mismatch.
3. **Markers.** Serial: `perf: fat-read ticks=<n>`, `perf: memfs-read ticks=<n>`, `perf: fs-read-delta fat=<a> memfs=<b>`. No ratio percent; no “faster/slower” words on the serial line. Missed samples print `perf: fat-read missed` / `perf: memfs-read missed`.
4. **Existing markers unchanged.** Keep `fat: read` / `fat: write` / ADR-051 write pair / A9 `slot:*` / ADR-046 pair. Grow / readdir / delete / Track N untouched.
5. **Smoke.** `scripts/qemu-smoke.sh` greps the new markers and rejects `fs-read-delta` lines that claim percent/faster/slower.
6. **Honesty.** This is a **measurement pair** on QEMU `virt` TCG. It is not a latency budget, not a product KPI, and not “FAT read cost is X.” Record Verified only with serial evidence on a named tip. Do not claim app hosting done, EL0 isolated, POSIX, or FAT32.

## Consequences

- Code: `src/fat.rs` (CNTPCT around FAT read + memfs compare + markers); smoke greps; `#[test_case]` that both samples are `> 0`.
- Docs: this ADR, [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, roadmap `P-PERF-7`, threat-model light bump.
- Follow-ups (not this PR): yield/SVC CNTPCT pair, memcpy baseline, multi-cluster read cost — only after a probe shows a need.
