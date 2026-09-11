# Daily brief — 2026-09-11 (ADR-019 identity text range)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/27 (`cursor/identity-tear-text-range-ea15`). Parent is `main` `b0f0ee5` (Merge PR #26). One PR: ADR-019 high-VA continuation + 16 KiB dedicated identity text range. No new FR/NFR IDs. PAN unclaimed.

Cloud `scripts/qemu-smoke.sh` **Verified** on `3e710e1` (2026-09-11, QEMU 8.2.2, `rustc` 1.100.0-nightly `67eda617e`): host `perf: elf-size bytes=3905712`; `ident: jump` / `ident: range lo=0x400ac000 hi=0x400b0000 pages=4` / `ident: text` plus prior ident/ttbr1/standing/asid/paging/heap/sched/wx/guard/ro/el0/perf markers; hello BRK ELRs high; `Running 48 tests` all `[ok]`; force-fail exit 1. GHA on `ac3ad0b` Failed (`ident: probe missed`); this SHA is the 16 KiB + high-VA flag publish.

First attempt to unmap live `.text` after `_start` **Failed** (unhandled sync on `println!` — rustc `dyn Write` vtables are identity fn pointers). Live `.text` stays. `_start` stays at `0x4008_0000`. Full identity teardown stays **Planned**.

Sponsor cts-ai Docker on `b0f0ee5` recorded Verified in the ledger (47 tests, `ident: ok`). Not this cloud VM.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #27 is `artofdream`; merge hat is `cursor[bot]` after this-run green checks.
2. Do not claim “EL0 isolated” or “the kernel moved.” Next identity work is live `.text` only after fmt/`dyn` is proven high-only — not a silent yank.
3. PAN still needs `ID_AA64MMFR1_EL1.PAN != 0`. Do not switch `-cpu`.

## Honesty

- Cloud Verified is this QEMU virt guest on this revision.
- Did not claim PAN or that identity `.text` after `_start` was fully torn down.
