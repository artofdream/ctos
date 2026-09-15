# `yield-libctos` — freestanding EL0 cooperative-yield sample

Fourth freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-062](../../docs/03-adr/ADR-062-yield-libctos-sample.md)). Complements [`hello-libctos`](../hello-libctos/README.md) (UART + **one** yield), [`fs-libctos`](../fs-libctos/README.md) (memfs), and [`fat-libctos`](../fat-libctos/README.md) (FAT `/probe`).

This is **not** a hosted app, not glibc, not POSIX, not preemption, not multi-task EL0, not a process table.

## What it prints

`main` writes `libctos: yld-hi`, then several `libctos: beat` lines across `yield_now()` rounds, then `libctos: yld-ok` and returns 0. Smoke also greps kernel `yldemo: ok` from the FAT `/yldemo` load path.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/yield-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/yldemo` beside `/hello`, `/fsdemo`, and `/fatdemo`. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/yield-libctos/Cargo.toml \
  --target user/yield-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No preemptive timer slice — cooperative `yield_now()` only.
- No second EL0 task / process table.
- No filesystem SVCs (see `fs-libctos` / `fat-libctos`).
- No argv, environ, or libc.
- Does not replace `/hello`, `/fsdemo`, or `/fatdemo` — all four slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
