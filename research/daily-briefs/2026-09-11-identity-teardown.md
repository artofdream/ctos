# Daily brief — 2026-09-11 (identity-tear first cut)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/26 (`cursor/identity-teardown-c0fd`). Parent is `main` `008f7c7` (Merge PR #25). One PR: ADR-018 split TTBR1 RAM tables + one torn identity text page. No new FR/NFR IDs. PAN unclaimed.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3895632`; `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok` plus prior ttbr1 / standing / asid / paging/heap/sched/wx/guard/ro/el0/perf markers; `Running 47 tests` all `[ok]`; force-fail exit 1.

`_start` / QEMU `-kernel` stay at `0x4008_0000`. Full identity teardown stays **Planned**. Umbrella isolation stays **Planned**.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #26 is expected `cursor[bot]`; merge hat is `artofdream`.
2. Do not claim “EL0 isolated” or “the kernel moved.” Next isolation work is PAN (needs `ID_AA64MMFR1_EL1.PAN != 0` — not a silent `-cpu` switch) or unmap more identity after a complete high-VA jump.
3. Remaining identity `.text`/`.data`/heap still mapped. The torn page is only `__ident_tear_*`.

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Did not claim PAN or that identity mappings were fully torn down.
