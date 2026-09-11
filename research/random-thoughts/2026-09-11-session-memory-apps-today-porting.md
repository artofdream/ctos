# Session memory — 2026-09-11 (apps-today + porting pages)

Docs + `research/` only on `cursor/docs-refresh-post-27-192e`. Branch already on `main` `e80dc93` (ADR-020 Verified). Did not rebase again.

Sponsor asked for first-class pages, not overview one-liners:

- Samples: M9 coop UART workers; heartbeat/counter called out as a **variant** (same `spawn`/`yield` shape, no `sched: beat` marker today); M6 RX echo of injected `0x41`; standing dual-SVC enter/leave. Cannot-run names Linux ELF, shell, Python, network, FS, SMP.
- Porting: easiest path is in-tree `no_std` on `aarch64-ctos.json` + cargo/docker-smoke. POSIX/glibc not easy. Later Planned: SVC ABI then a freestanding `libctos` (name reserved; no crate). Isolation miles first.
- Overview kept KPIs / prerequisites / advantages / drawbacks.
- Second brain: new daily brief + this note; moc + research README point at the new pages. No `.obsidian/`.

Idle-tip facts unchanged: Docker Verified `e80dc93` (50 tests, reloc n=12, live pages=37); Pages CNAME exists, site **Planned**; do not claim https://ctos.artof.link works.

Did not touch `src/`, `linker.ld`, or smoke scripts. Do not treat this file as the honesty ledger.
