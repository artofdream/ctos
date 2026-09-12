# Session memory — 2026-09-12 (Track B / B5 Linux VFS vs thin ctos VFS)

Docs + `research/` only on `cursor/b5-linux-vfs-vs-thin-ctos-f294`. Rebased onto `main` `d81a53a` (B3 / #67 ADR-035 + B4 / #68 ADR-033).

- **B5 / #45:** ADR compare, not implementation. How the thin VFS stays thin; optional later mapping is translation after B6, not a dentry cache.
- Pairing: **ADR-033** = B4 ELF. **ADR-034** = this VFS compare (frozen). **ADR-035** = B3 process model (merged). Do not rename again.
- Must cite ADR-027 / ADR-028 / ADR-031 and the B2 `openat`/`read`/`write`/`close` rows. Must not grow POSIX flags / dentries / cwd / `mount(2)`.
- Status words reused from B2: **present** / **partial** / **absent** / **never-per-ADR-031**. No Linux VFS object is `present`.
- Did not mint FR/NFR IDs. Did not rewrite large ledger tables (append-only rows). B3 + B4 ledger rows from main kept.
- Did not run kernel `qemu-smoke` (no `src/` edit).
- GitHub lists PR #66 as `artofdream`-opened (cloud tool). Merge hat is `cursor[bot]`. Do not self-merge.

Do not treat this file as the honesty ledger.
