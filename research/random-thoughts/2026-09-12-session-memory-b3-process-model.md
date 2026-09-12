# Session memory — 2026-09-12 (Track B / B3 process model)

Docs + `research/` only on `cursor/b3-process-model-3443`. Rebased onto `main` `774c672` (B4 / #68).

- **B3 / #43:** ADR compare, not implementation. Issue asked to decide approach without abandoning probes.
- Frozen pairing after the Track B ADR race: B4 = **ADR-033** (ELF, on `main`); B5 / #66 = ADR-034; this mile = **ADR-035**. Do not rename again.
- Chosen H1: standing EL0 stays the ctos process model. Linux `clone`/`execve`/`wait4` is a documented gap. Do not add those syscalls. Do not rewrite A1–A9.
- Rejected: promising fork/exec/wait; treating A4/A9 as `execve`; treating M9 workers as Linux threads; adding clone/wait as a Track B backlog; reopening `CLONE_NEW*`.
- Linux AArch64 has no `fork` syscall (B2 already said that). Userspace `fork()` is `clone`.
- Did not implement B4–B6. Did not mint FR/NFR IDs. Did not rewrite large ledger tables (append-only rows).
- Did not run kernel `qemu-smoke` (no `src/` edit).

Do not treat this file as the honesty ledger.
