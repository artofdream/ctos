# Daily brief — 2026-09-12 (Track B / B3 process model)

## Where we stopped

Rebased PR on `cursor/b3-process-model-3443` onto `main` `774c672` (B4 / #68, ADR-033 ELF). One docs-only PR: [ADR-035](../../docs/03-adr/ADR-035-process-model-standing-el0.md) compares Linux `fork`/`exec`/`wait` (AArch64: `clone`/`execve`/`wait4`) to ctos standing EL0 ([ADR-024](../../docs/03-adr/ADR-024-standing-el0-normal.md)) and EL1 coop yield ([ADR-010](../../docs/03-adr/ADR-010-cooperative-rr-el1.md)). Stance: keep standing EL0; do not add those syscalls; do not rewrite A1–A9 probes. Cites [ADR-031](../../docs/03-adr/ADR-031-linux-compat-goals.md). Explicit **not claiming Linux userspace yet**. No new FR/NFR IDs. No B5–B6 implementation. No `src/` change.

Frozen pairing: B4 / #68 = ADR-033 (ELF, now on `main`); B5 / #66 = ADR-034 (VFS, still open); this PR B3 = **ADR-035**. Do not rename again.

B3 is **Documented** on [track-b.md](../../docs/04-roadmap/track-b.md). B4 stays **Documented** from main. Scratch: [random-thoughts/2026-09-12-session-memory-b3-process-model.md](../random-thoughts/2026-09-12-session-memory-b3-process-model.md).

## Do next

1. Separate MRC session (`COMMENT` only). Author does not merge (ADR-002). Cloud PRs here list as `artofdream`-opened; merge hat is `cursor[bot]` after this-run green checks (when CI exists) and Bugbot resolved-or-declined.
2. Close [issue #43](https://github.com/artofdream/ctos/issues/43) after merge (`Closes #43` is on the PR).
3. B4 ([#44](https://github.com/artofdream/ctos/issues/44)) is already **Documented** on `main` as [ADR-033](../../docs/03-adr/ADR-033-linux-elf-auxv-pt-interp.md). Do not accept `PT_INTERP`.

## Honesty

- Docs + file-read of ADR-024 / ADR-010 / ADR-021 / B2 gap map / ADR-033 ELF only. Did not run QEMU. Did not claim Linux userspace, a process table, or a new Pages deploy.
- Standing EL0 is not `execve`. A9 FAT `/hello` is an EL1 load path. `SYS_EXIT` is not `exit_group`. EL1 coop workers are not Linux threads.
- Local docs-build is a later row on this PR after `./scripts/docs-build.sh`. Generator only.
- Ledger: B4 rows from main kept; B3 ADR-035 inspection row appended. Existing tables not rewritten. “Guest runs host apps” stays **Planned**.
