# Session memory — 2026-09-12 (Track B / B1 Linux-compat goals)

Docs + `research/` only on `cursor/b1-linux-compat-goals-9374`. Rebased onto `main` `4459b93` after #61 (ADR-002 historical exception). Ledger conflict: kept #61 identity + Historical rows, then B1 rows.

- **B1 / #41:** write the Track B frame. Linux-compat = research into a possible Linux AArch64 ABI **subset**, not a distro.
- Next free ADR after ADR-030 was **ADR-031**.
- Must cite ADR-029 (B7 already merged). Must not reopen containers as Planned / far-later.
- Must keep ctos specificity: honesty, antifragility, security, performance, freestanding Track A path, virt scope.
- Explicit sentence: **not claiming Linux userspace yet.** A3 rejects `PT_INTERP`; A9 FAT `/hello` is not `execve`.
- Did not implement B2–B6. Did not mint FR/NFR IDs. Did not rewrite large ledger tables (append-only rows).
- Did not run kernel `qemu-smoke` (no `src/` edit).
- GitHub lists PR #62 as `artofdream`-opened (cloud tool). Merge hat is `cursor[bot]`. Do not self-merge.

Do not treat this file as the honesty ledger.
