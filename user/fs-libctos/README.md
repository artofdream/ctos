# `fs-libctos` — freestanding EL0 VFS sample

Second freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-059](../../docs/03-adr/ADR-059-fs-libctos-sample.md)). Complements [`hello-libctos`](../hello-libctos/README.md) (UART + yield only).

This is **not** a hosted app, not glibc, not POSIX, not `getdents` from EL0.

## What it prints

`main` writes `libctos: fs-hi`, yields, then `fs_create` / `fs_write` / `fs_close` / `fs_open` / `fs_read` / `fs_close` on `/memdemo` (ADR-058: longest prefix `/mem` → memfs; flat path grammar — no `/mem/...`). On success it writes `libctos: fs-ok` and returns 0. Smoke also greps kernel `fsdemo: ok` from the FAT `/fsdemo` load path.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/fs-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/fsdemo` beside `/hello`. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/fs-libctos/Cargo.toml \
  --target user/fs-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## EL0 data vs exec pages

User `PT_LOAD` RX pages are mapped fetch-only for EL0 (`paging::l3_page_el0_exec`: no EL0 load/store). String literals may be **passed to SVCs** (EL1 copies them) but must not be `LDR`'d at EL0. This sample compares the read-back with immediates.

## What this payload does not do

- No Linux/POSIX filesystem ABI. Thin VFS SVCs only (16–23).
- No `getdents` / directory listing from EL0.
- No argv, environ, or libc.
- Does not replace `/hello` — both slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
