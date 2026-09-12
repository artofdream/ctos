# Session memory — 2026-09-11 (Track A / A1 SVC ABI)

Rebased onto `origin/main` `aa46219` (#29 after #51). Conflicts on first commit: ledger, el0, pillars, security, technical-architecture, roadmap. Kept #29 extras + #51 HTTPS Verified + A1 ABI rows. Folded A1 into `track-a.md` and extras without claiming hosting. Skipped stale `c9bf98f` smoke-record commit.

Bugbot Medium on `user_range_ok`: walks mask to 39 bits. Fixed by requiring `ptr`/`last == identity_pa(...)`. New test `el0_uart_write_rejects_high_alias`.

qemu-smoke ok on `09425c0`. 54 tests. `perf: elf-size bytes=4020520`. PR #49 ready. Do not self-merge. Hats: `artofdream` → `cursor[bot]`.
