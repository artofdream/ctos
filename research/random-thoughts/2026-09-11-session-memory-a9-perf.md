# Session memory — 2026-09-11 (A9 performance shape)

Docs + `research/` only on PR #29.

Sponsor: performance impact of OS/app slot disconnect (A9).

- **Costs (hypothesis):** boot/load, SVC boundary, ASID/TTBR switches, optional COW later.
- **Neutral/wins:** steady EL0 compute similar once mapped; smaller OS updates are operational, not a CNTPCT claim.
- **Gate:** keep `perf: boot-delta`; add `perf: app-load` (or similar) when a loader exists. Fail closed.
- **Today:** one linked ELF. **No Verified delta.** No invented benches.

Wrote under `docs/framework/performance.md` + overview KPI / A9 bullets + ledger Planned row. Do not treat this file as the ledger.
