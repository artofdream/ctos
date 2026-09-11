# Daily brief — 2026-09-11 (ASID isolation mile)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/23 (`cursor/el0-asid-isolation-f98d`). Parent is `main` `7dbd7e6` (Merge PR #22). One PR: Docker Verified on `71ee15f` + ASID TLB mile. No new FR/NFR IDs.

A — Sponsor cts-ai Docker on `71ee15f`: **Verified** (`elf-size bytes=3838592`, layout + all hello probes, 38 tests, EXIT 0). Keep `b2bbb99` Failed.

B — Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3853144`; `asid: dual` / `asid: conflict` / `asid: ok`; `Running 39 tests` all `[ok]`; force-fail exit 1.

Standing EL0 deferred. PAN unclaimed. Umbrella isolation stays **Planned**.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #23 is expected `cursor[bot]`; merge hat is `artofdream`.
2. Isolation still needs PAN / standing EL0 / TTBR1. Do not claim “EL0 isolated.”
3. EL0 trampoline still `TLBI VMALLE1` (kernel `.data` is global). Next mile if wanted: `nG` on `.data` so that path can drop the full flush.

## Honesty

- Docker Verified is the sponsor serial on `71ee15f`, not this VM.
- Cloud Verified is this QEMU virt guest on the dual-ASID `nG` probe.
- Did not claim PAN.
