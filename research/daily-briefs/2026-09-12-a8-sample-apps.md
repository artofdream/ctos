# Daily brief — 2026-09-12 (Track A / A8 documented sample apps)

## Where we stopped

Draft PR on `cursor/a8-documented-sample-apps-6be5` from `main` `ae7d2b8` (A7 / ADR-028). Parent [#31](https://github.com/artofdream/ctos/issues/31), child [#39](https://github.com/artofdream/ctos/issues/39).

**Docs-only.** No new ADR (no new ABI, loader, or slot decision). Rebuild recipes for Verified classes only:

- Cooperative UART workers (EL1) — `sched:*`
- UART RX echo — `input: rx 0x41`
- Standing EL0 / `libctos` hello (loader path) — `libctos:*` / `loader:*` / `el0: task-*`
- Optional: memfs (`fs:*`) and FAT16 `/probe` (`blk:*` / `fat:*`)

Hub: [what-can-run.md](../../docs/overview/what-can-run.md). Walkthroughs: [apps-today.md](../../docs/framework/apps-today.md). In-tree: `user/README.md`, `user/hello-libctos/README.md`.

Cloud `./scripts/docs-build.sh` **Verified** on `f93d5de` (`mdbook v0.5.4`, mermaid 0.17.1, `docs-build: ok`, `book/CNAME` `ctos.artof.link`). Kernel `qemu-smoke` not re-run (no `src/` edit). A1–A7 smoke markers unchanged.

No new FR/NFR IDs. No “app hosting done.” A9 stays **Planned**.

## Do next

1. Human or MRC review. Author does not merge (ADR-002).
2. **A9** ([#48](https://github.com/artofdream/ctos/issues/48)): OS image vs app payload disconnect — two artifacts, a load path that is not `include_bytes!` into one ELF, and a cross-update probe (same app on OS n and n+1). Today is still one linked ELF.

## Honesty

- A8 Verified is “recipes match existing probes” + docs-build. File presence of a README is not QEMU boot.
- Did not add a `libctos` hello that calls `fs_open("/probe")`. Wrappers exist; the hello does not use them. EL0 VFS stays `/eprobe`.
