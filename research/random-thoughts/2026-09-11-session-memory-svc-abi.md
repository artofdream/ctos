# Session memory — 2026-09-11 (Track A / A1 SVC ABI)

Rebased onto `origin/main` `f86785b` (#30 Pages). Conflicts: `roadmap.md` and `honesty-ledger.md` only. Kernel files did not conflict. Branch `cursor/svc-syscall-abi-0dc5`. Draft PR #49. Do not `git pull` the old remote tip.

ADR-021: public `SVC #16` exit / `#17` uart_write / `#18` yield. Reserved 0–2 stay ADR-013 probes (tightened first-mile to imm==0 so it cannot steal ABI numbers). `yield` does not call `sched::yield_now` (exception frame / ADR-010). `uart_write` copies ≤64 bytes from a user-mapped **and** kernel-mapped range; kernel `.data` returns 0.

Payload on `EL0_PAGE`: SVC #18, MOVZ/MOVK user ptr, SVC #17, SVC #16. User buffer `svc: user-hi\n`.

53 tests. qemu-smoke ok on `79afb57`. Track A incomplete.

Do not self-merge. GitHub author of #49 is `artofdream`; merge hat is `cursor[bot]`. This session does not merge.
