# `mkdir-libctos` — freestanding EL0 `fs_mkdir` sample

Seventh freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-075](../../docs/03-adr/ADR-075-el0-fs-mkdir.md)). Complements [`hello-libctos`](../hello-libctos/README.md), [`fs-libctos`](../fs-libctos/README.md), [`fat-libctos`](../fat-libctos/README.md), [`yield-libctos`](../yield-libctos/README.md), [`net-libctos`](../net-libctos/README.md), and [`udp-libctos`](../udp-libctos/README.md).

This is **not** a hosted app, not glibc, not POSIX `mkdir`/`mkdirat`, not a nested directory tree, and not exFAT/FAT32.

FAT 8.3 forces a short slot name: VFS path `/mkdemo` (not `/mkdirdemo`, which is nine characters).

## What it prints

`main` writes `libctos: mkdir-hi`, creates FAT root `/edir` via `fs_mkdir` (SVC 27), proves `Exists` on a second create and `BadPath` on `/bad/nest`, then `libctos: mkdir-ok` and returns 0. Smoke also greps kernel `mkdemo: ok` from the FAT `/mkdemo` load path.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/mkdir-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/mkdemo` beside the prior six samples. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/mkdir-libctos/Cargo.toml \
  --target user/mkdir-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No POSIX `mkdir` / `mkdirat` / directory trees / recursive create.
- No nested-path claim (BadPath is fail-closed).
- No argv, environ, or libc.
- Does not replace `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, `/netdemo`, or `/udpdemo` — all seven slots stay on the image.
- Does not add EL0 TCP.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
