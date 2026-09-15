# ADR-061 — Third freestanding sample (`fat-libctos`) — FAT via thin VFS

- Status: Accepted (sample + smoke Verified when serial / tests pass; product freestanding app hosting remains Verified under ADR-048/052 — do not reopen)
- Date: 2026-09-15

## Context

[ADR-059](ADR-059-fs-libctos-sample.md) added a second freestanding EL0 sample that exercises libctos VFS on **memfs** (`/memdemo` via ADR-058 `/mem` prefix). Product gap remains: no freestanding sample that opens/reads a **FAT** path from user mode through the same thin VFS (`/` → FAT16). Kernel `/probe` (`fat-hi`) write/readdir/delete are already Verified at the EL1 path.

This mile adds a **third freestanding EL0 sample** that reads FAT `/probe` via libctos, publishes it on FAT beside `/hello` and `/fsdemo`, expands the catalog, and fail-closes smoke on new markers. Honesty: not POSIX, not Linux, not “EL0 isolated.”

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | New crate `user/fat-libctos`. Open/read/close FAT `/probe` expecting `fat-hi`. FAT slot `/fatdemo` (8.3 `FATDEMO`). Kernel loads after ADR-059 fsdemo. Serial `libctos: fat-hi` / `libctos: fat-ok` + `fatdemo: ok`. ADR + catalog docs. | Cleanest deeper product; keeps `/hello` + `/fsdemo` intact. |
| **H2 (rejected)** | Teach `fs-libctos` to also open `/probe`. | Collapses memfs vs FAT recipes; muddies ADR-059 markers. |
| **H3 (rejected)** | EL0 create/write/read a new FAT root name this mile. | Prefer existing `/probe` (already restored to `fat-hi` before this probe). Write-to-FAT from EL0 is already routed but not required for the product claim. |
| **H4 (rejected)** | Replace `/fsdemo` or `/hello`. | Breaks A9 / ADR-059 cross-update and markers. |
| **H5 (rejected)** | Claim POSIX `open` / Linux VFS / “EL0 isolated.” | Honesty forbid. |

## Decision

1. **Sample crate.** `user/fat-libctos`: same `aarch64-ctos-user` / `linker.ld` / `build-std` as hello/fs. `main` prints `libctos: fat-hi`, yields, then `fs_open` / `fs_read` / `fs_close` on `/probe`, then `libctos: fat-ok`. Fail path prints `libctos: fat-fail` and exits non-zero.
2. **Path choice.** `/probe` resolves to FAT16 under ADR-058 (`/` → FAT). Payload bytes are the existing A7 `fat-hi` (restored by `fat::observe_probe` before this slot runs).
3. **Publish.** `build.rs` builds and publishes `target/fat-libctos.elf`. `scripts/mkfat16.py --app3` writes 8.3 `FATDEMO`. Keep `/hello` + `/fsdemo` + `slot: ok` + `fsdemo: ok`.
4. **Kernel probe.** `src/fatdemo.rs` loads FAT `/fatdemo`, `ERET`s via `loader::run_image_expecting` (last uart len 16). Serial `fatdemo: fat` / `fatdemo: mapped` / `fatdemo: ok`. Runs after the ADR-059 fsdemo probe.
5. **Smoke.** Fail-closed greps for the new markers; reject `libctos: fat-fail`; preserve all existing greps.
6. **Docs.** This ADR; honesty ledger evidence-only; `user/README.md`; [what-can-run.md](../overview/what-can-run.md); [apps-today.md](../framework/apps-today.md); SUMMARY. Threat-model **v1.36** (NFR-10 text in place).
7. **EL0 exec pages.** RX user pages are fetch-only for EL0. The sample may pass `.rodata` pointers to SVCs (EL1 copy) but must not `LDR` them at EL0; read-back compare uses immediates.
8. **Honesty.** Do **not** claim: EL0 isolated, PAN enable, taken SError, Linux/POSIX/containers, immutable-OS marketing, or reopen “app hosting is done.” Never yank `_start`. No silent `-cpu`.

## Consequences

- Code: `user/fat-libctos/`, `build.rs`, `scripts/mkfat16.py`, `scripts/qemu-aarch64.sh`, `scripts/qemu-smoke.sh`, `src/fatdemo.rs`, `src/main.rs`.
- Docs: this ADR + catalog / ledger / threat-model v1.36.
- Follow-ups (not this PR): EL0 FAT create/write sample, EL0 readdir when wired, tree mounts with an extra `/`.
