# Session memory — 2026-09-09 (ADR-011 pillars)

Started from stale local `main` (M6). Fetched `origin/main` `4e3b732` (M9 merged). Next free ADR was 011.

W^X on the heap is not a small ratchet: ADR-008 maps RAM as one executable 1 GiB L1 block; heap + coop stacks live there. Setting PXN on the M7 L3 window would be a fake “NX heap” claim. Left Planned.

Landed option B: `src/perf.rs` CNTPCT delta around a 10k `black_box` add loop; smoke greps `perf: cntpct`. Not a bench.

NFR IDs frozen. Text only under ADR-011: NFR-05 Must/Now, NFR-07 Should/Now, NFR-10 stub + claim gate.

Do not self-merge. Do not mint FR-16+ / NFR-15+. GitHub author of #17 is `artofdream`; merge hat is `cursor[bot]` (ADR-002). MRC COMMENT noted #16 ledger collision — fold M9 GHA URLs rather than leave that row Unknown.
