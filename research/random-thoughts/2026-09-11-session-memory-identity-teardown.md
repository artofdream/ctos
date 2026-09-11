# Session memory — 2026-09-11 (identity-tear first cut)

Fetched `origin/main` `008f7c7` (#25 merged). Branch `cursor/identity-teardown-c0fd`. Draft PR #26.

ADR-018: clone `L2_RAM` (+ L3s) into `L2_HIGH_RAM` / `L3_HIGH_RAM`. `L1_HIGH[1]` points at the clone. Dedicated 4 KiB `ident_tear` section at `0x400ab000` (`ident_tear_el1_path`). After MMU + high VBAR, unmap that page from identity and user TTBR0; `TLBI VAAE1` the low VA. High twin stays.

Hello serial: `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok`. 47 tests. Fatal nested ELR `0xffffff8040083bec` (high). Hello BRK ELRs stayed identity (`0x40082740` / `0x40082718`).

Did not unmap `_start` (`0x40080000`) or the rest of identity `.text`/`.data`/heap. rustc static relocation still needs those VAs.

PAN unclaimed. Full teardown Planned. Do not self-merge. GitHub author of #26 is expected `cursor[bot]`; merge hat is `artofdream`.
