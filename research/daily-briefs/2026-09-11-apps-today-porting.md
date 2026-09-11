# Daily brief — 2026-09-11 (first-class apps-today + porting)

## Where we stopped

`main` ≈ `e80dc93` (Merge PR #28 / ADR-020). Docs + `research/` only on `cursor/docs-refresh-post-27-192e` (PR #29). No `src/` / `linker.ld` / smoke-script changes. This PR does **not** publish GitHub Pages.

First-class docs (not overview one-liners):

- [docs/framework/apps-today.md](../../docs/framework/apps-today.md) — coop EL1 UART workers (`sched: task a/b/ok`); heartbeat/counter as a same-shape **variant** (not in tree); UART RX echo (`input: rx 0x41`); standing EL0 SVC enter/leave; cannot-run list (Linux ELF, shell, Python, network, FS, SMP).
- [docs/framework/building-or-porting.md](../../docs/framework/building-or-porting.md) — easiest = in-tree `no_std` coop EL1 on `aarch64-ctos.json` + `cargo` / `qemu-smoke` / `docker-smoke`; POSIX/glibc not easy and not started; later **Planned** SVC ABI + `libctos` for freestanding EL0 (no crate today).
- [docs/framework/overview.md](../../docs/framework/overview.md) keeps KPIs, prerequisites, advantages, drawbacks and points at those pages.

Linked from README, vision, pillars, el0, architecture, [moc.md](../moc.md).

ADR-020 is **Verified** on `e80dc93` (`ident: reloc` / `ident: live`). Sponsor Docker **Verified** on that SHA: 50 tests, `ident: reloc n=12`, live pages=37, force-fail. GHA merge [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) grepped. Keep `24d94e6` Docker Verified and `b2bbb99` Failed.

Route 53 CNAME `ctos.artof.link` → `artofdream.github.io.` exists in zone `Z1178AFMV41RWP` (account `737290977112`). Custom-domain reachability stays **Planned**. Do not claim https://ctos.artof.link works. `.obsidian/` not committed.

## Do next

1. MRC `COMMENT` on PR #29 in a **new** session. Author does not merge (ADR-002). Merger: `cursor[bot]`.
2. Still **Planned**: `.rodata`/`.data`/heap tear, PAN on `cortex-a57`, umbrella EL0 isolation, Pages reachability, SVC ABI / `libctos`.
3. Do not claim “secure OS,” “the kernel moved,” “EL0 isolated,” or “apps run.”

## Honesty

- Docker `e80dc93` is a sponsor probe. This session did not run QEMU on this VM.
- Heartbeat/counter and `libctos` are named as **Planned** / not in tree.
