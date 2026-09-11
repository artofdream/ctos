# Daily brief — 2026-09-11 (standing EL0 + TTBR1 first cut)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/24 (`cursor/el0-standing-ttbr1-5ce0`). Parent is `main` `fb050b7` (Merge PR #23). One PR: standing EL0 + ADR-016 TTBR1 private page. No new FR/NFR IDs. PAN unclaimed.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3872120`; `el0: standing` / `el0: restored`; `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`; prior paging/heap/sched/wx/guard/ro/el0/asid/perf markers; `Running 42 tests` all `[ok]`; force-fail exit 1.

Umbrella isolation stays **Planned** (PAN + full higher-half / identity teardown still missing).

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #24 is expected `cursor[bot]`; merge hat is `artofdream`.
2. Do not claim “EL0 isolated.” Next isolation work is PAN (needs `ID_AA64MMFR1_EL1.PAN != 0` on the probe CPU — not a silent `-cpu` switch) or a real higher-half relocate.
3. Lower-EL IRQ still parks. Standing mile is dual-SVC, DAIF masked.

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Did not claim PAN or that the kernel moved to a high VA.
