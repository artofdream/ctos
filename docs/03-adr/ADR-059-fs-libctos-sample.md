# ADR-059 — Second freestanding sample (`fs-libctos`) + catalog expansion

- Status: Accepted (sample + smoke Verified when serial / tests pass; product freestanding app hosting remains Verified under ADR-048/052 — do not reopen)
- Date: 2026-09-14

## Context

Track A / A8 documented rebuild recipes for in-tree samples ([what-can-run.md](../overview/what-can-run.md), `user/README.md`). Today only one freestanding EL0 sample exists: [`user/hello-libctos`](../../user/hello-libctos/) (UART + yield). Docs called out the gap: a `libctos` hello that calls `fs_open` was not in the tree. Wrappers exist (SVC 16–23); the Verified EL0 VFS trip was the kernel `/eprobe` trampoline.

After [ADR-058](ADR-058-vfs-prefix-mounts.md), longest-prefix mounts route `/mem` (+ A6 names) → memfs and `/` → FAT16. Flat path grammar unchanged (`/` + `[a-z0-9_-]`, no extra `/`).

This mile adds a **second freestanding EL0 sample** that exercises libctos VFS, publishes it on FAT beside `/hello`, expands the catalog, and fail-closes smoke on new markers. Honesty: not POSIX, not Linux, not `getdents` from EL0.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | New crate `user/fs-libctos` mirroring hello target/linker/build-std. Create/write/read/close `/memdemo` (memfs via `/mem` prefix). FAT slot `/fsdemo` (8.3 `FSDEMO`). Kernel loads after A9 hello. Serial `libctos: fs-hi` / `libctos: fs-ok` + `fsdemo: ok`. ADR + catalog docs. | Cleanest fit; keeps `/hello` intact. |
| **H2 (rejected)** | Teach `hello-libctos` to call `fs_open` in-place. | Collapses two recipes into one; muddies UART-only vs VFS samples. |
| **H3 (rejected)** | Use `/mem/...` tree path or claim POSIX `open`. | Breaks flat grammar / honesty. |
| **H4 (rejected)** | Replace `/hello` with the VFS sample. | Breaks A9 cross-update and existing markers. |
| **H5 (rejected)** | Add EL0 `getdents` this mile. | Not wired; do not invent. |

## Decision

1. **Sample crate.** `user/fs-libctos`: same `aarch64-ctos-user` / `linker.ld` / `build-std` as hello. `main` prints `libctos: fs-hi`, yields, then `fs_create` / `fs_write` / `fs_close` / `fs_open` / `fs_read` / `fs_close` on `/memdemo`, then `libctos: fs-ok`. Fail path prints `libctos: fs-fail` and exits non-zero.
2. **Path choice (hunch → verify).** `/memdemo` longest-prefix-matches `/mem` → memfs under ADR-058. Flat grammar forbids `/mem/foo`. The **ELF** lives on FAT `/fsdemo`; the **in-app file** is memfs.
3. **Publish.** `build.rs` builds and publishes `target/fs-libctos.elf`. `scripts/mkfat16.py --app2` writes 8.3 `FSDEMO`. Keep `/hello` + `slot: ok`.
4. **Kernel probe.** `src/fsdemo.rs` loads FAT `/fsdemo`, `ERET`s via `loader::run_image_expecting` (last uart len 15). Serial `fsdemo: fat` / `fsdemo: mapped` / `fsdemo: ok`. Runs after the A9 slot probe.
5. **Smoke.** Fail-closed greps for the new markers; preserve all existing greps (hello, FAT write/readdir/delete, mounts, A9 cross-update, …).
6. **Docs.** This ADR; honesty ledger evidence-only; `user/README.md`; [what-can-run.md](../overview/what-can-run.md); [apps-today.md](../framework/apps-today.md); SUMMARY. Threat-model **v1.34** (NFR-10 text in place).
7. **EL0 exec pages.** RX user pages are fetch-only for EL0 (existing `l3_page_el0_exec`). The sample may pass `.rodata` pointers to SVCs (EL1 copy) but must not `LDR` them at EL0; read-back compare uses immediates.
8. **Honesty.** Do **not** claim: EL0 isolated, PAN enable, taken SError, Linux/POSIX/containers, immutable-OS marketing, or reopen “app hosting is done.”

## Consequences

- Code: `user/fs-libctos/`, `build.rs`, `scripts/mkfat16.py`, `scripts/qemu-aarch64.sh`, `scripts/qemu-smoke.sh`, `src/fsdemo.rs`, `src/loader.rs` (`run_image_expecting`), `src/main.rs`.
- Docs: this ADR + catalog / ledger / threat-model v1.34.
- Follow-ups (not this PR): richer samples, EL0 readdir when wired, tree mounts with an extra `/`.
