# Session memory — 2026-09-10 (pillars follow-up)

Fetched `origin/main` `0b1339d` (ADR-011 merged). One PR, not a stack: sponsor asked to avoid conflicting parallel branches.

W^X: cannot use `SCTLR.WXN` while text is writable. Replaced RAM L1 block with L2 + one L3 for the 2 MiB that straddles `__kernel_end`. Only 128 MiB mapped. Heap IABORT: arm flag, `blr` to heap `RET`, resume at LR. IFSC permission L1/L2/L3. Serial `wx: nx heap` then `wx: ok`.

Coop stacks are heap `Vec`s so heap PXN covers them. Linker stacks stay X — say so.

IRQ-delta: read `CNTP_CVAL` *before* `write_tval` in `on_interrupt`. `observe_ticks` now re-arms so a second test window works. Hello uses 8 ticks.

EL0: stub + ADR-013 only. `is_active() == false` test.

Obsidian: checklist page; `.gitignore` already had `.obsidian/`.

Do not self-merge. GitHub author of #18 TBD from the PR page; merge hat is the other identity (ADR-002).
