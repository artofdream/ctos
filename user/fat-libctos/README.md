# `fat-libctos` — freestanding EL0 FAT-via-VFS sample

Third freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-061](../../docs/03-adr/ADR-061-fat-libctos-sample.md); grow depth [ADR-064](../../docs/03-adr/ADR-064-fat16-multi-cluster-grow.md)). Complements [`hello-libctos`](../hello-libctos/README.md) (UART + yield) and [`fs-libctos`](../fs-libctos/README.md) (memfs `/memdemo` only).

This is **not** a hosted app, not glibc, not POSIX, not `getdents` from EL0.

## What it prints

`main` writes `libctos: fat-hi`, yields, then `fs_open` / `fs_read` / `fs_close` on FAT `/probe` (expects payload `fat-hi` via thin VFS `/` → FAT16). It then creates `/egrow`, writes 600 bytes across a cluster boundary via chunked `fs_write` (ADR-064), read-back proves boundary bytes, prints `libctos: fat-grow`, then `libctos: fat-ok`. Smoke also greps kernel `fatdemo: ok` / `fat: grow` from the FAT `/fatdemo` load path and kernel grow probe.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/fat-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/fatdemo` beside `/hello` and `/fsdemo`. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/fat-libctos/Cargo.toml \
  --target user/fat-libctos/aarch64-ctos-user.json
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

- No Linux/POSIX filesystem ABI. Thin VFS SVCs only.
- No POSIX write API. Create/write here is the thin VFS SVC trip that proves multi-cluster grow (ADR-064), not a product claim.
- No `getdents` / directory listing from EL0.
- No argv, environ, or libc.
- Does not replace `/hello` or `/fsdemo` — all three slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
