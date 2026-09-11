# Daily brief — 2026-09-11 (TTBR1 high-VA EL1 fetch)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/25 (`cursor/ttbr1-high-el1-exec-136c`). Parent is `main` `8745939` (Merge PR #24). One PR: ADR-017 EL1 fetch from the TTBR1 RAM alias + high VBAR. No new FR/NFR IDs. PAN unclaimed.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3877712`; `ttbr1: el1 exec` / `ttbr1: vbar` plus prior private-page / standing / asid / paging/heap/sched/wx/guard/ro/el0/perf markers; `Running 44 tests` all `[ok]`; force-fail exit 1.

`_start` / QEMU `-kernel` stay at `0x4008_0000`. Identity teardown stays **Planned**. Umbrella isolation stays **Planned**.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #25 is expected `cursor[bot]`; merge hat is `artofdream`.
2. Do not claim “EL0 isolated” or “the kernel moved.” Next isolation work is PAN (needs `ID_AA64MMFR1_EL1.PAN != 0` — not a silent `-cpu` switch) or split TTBR1 tables + identity teardown.
3. Handler `ADRP` must not be used as a TTBR/PA (`paging::identity_pa`).

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Did not claim PAN or that identity mappings were torn down.
