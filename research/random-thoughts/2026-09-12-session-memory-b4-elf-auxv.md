# Session memory — 2026-09-12 (Track B / B4 Linux ELF / auxv / PT_INTERP)

Docs + `research/` only on `cursor/b4-elf-auxv-pt-interp-51bc` from `main` `bc2af0b` (B2 / #65). PR #68.

- **B4 / #44:** ADR comparing Linux ELF `exec` (PT_INTERP + auxv + PT_DYNAMIC) vs freestanding A3 loader. Keep Track A loader reusable.
- Next free ADR after ADR-032 is **ADR-033** (B2 was a research note, not an ADR).
- ADR-031 already forbade accepting `PT_INTERP` in this child. Do not flip that.
- Static musl still needs Linux stack/auxv + B2 syscall surface. Dynamic musl/glibc also need the interpreter. Not a small subset. Not a promise.
- Did not implement B3/B5/B6. Did not mint FR/NFR IDs. Did not rewrite large ledger tables (append-only rows).
- Did not run kernel `qemu-smoke` (no `src/` edit).
- Cloud PRs typically show `author=artofdream`. Merge hat is `cursor[bot]`. Do not self-merge.

Do not treat this file as the honesty ledger.
