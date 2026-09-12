# Session memory — 2026-09-12 (Track B / B2 Linux AArch64 syscall gap)

Docs + `research/` only on `cursor/b2-linux-syscall-gap-c35b`. Branched from `main` `e6255c0` (B1 / ADR-031 / #62).

- **B2 / #42:** gap analysis, not implementation. Issue asked for `docs/research` + honesty: analysis Verified as a file, not ABI Verified.
- Next free ADR after ADR-031 would be ADR-032. **Not used.** B2 is not decision-grade. B6 (#46) is the compat-vs-never decision.
- Linux numbers from `include/uapi/asm-generic/unistd.h` **v6.10** (AArch64 generic table). ctos surface from `src/syscall.rs` + ADR-021 / ADR-027.
- Structural gaps: `svc #0`+`x8` vs `SVC #<n>`; number collisions 16–23; Linux `svc #0` hits ADR-013 probe.
- Required set covered: exit, write, read, openat, close, brk/mmap, clone/fork/execve/wait4, ioctl. Plus CRT/namespace neighbors.
- Did not implement B3–B6. Did not mint FR/NFR IDs. Did not rewrite large ledger tables (append-only rows).
- Did not run kernel `qemu-smoke` (no `src/` edit).

Do not treat this file as the honesty ledger.
