# `net-libctos` — freestanding EL0 net-SVC sample

Fifth freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-068](../../docs/03-adr/ADR-068-el0-net-svc-sample.md)). Complements [`hello-libctos`](../hello-libctos/README.md), [`fs-libctos`](../fs-libctos/README.md), [`fat-libctos`](../fat-libctos/README.md), [`yield-libctos`](../yield-libctos/README.md), and [`udp-libctos`](../udp-libctos/README.md).

This is **not** a hosted app, not glibc, not POSIX, not a TCP/UDP stack, not BSD sockets, not DHCP/DNS, not “has networking,” and not a guest virtio-net driver in EL0.

## What it prints

`main` writes `libctos: net-hi`, reads the guest MAC (`libctos: net-mac`), asks the kernel to ICMP-echo `10.0.2.2` (`net_ping`), then `libctos: net-ok` and returns 0. Smoke also greps kernel `netdemo: ok` from the FAT `/netdemo` load path.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/net-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/netdemo` beside `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, and `/udpdemo`. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/net-libctos/Cargo.toml \
  --target user/net-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No virtio-net programming from EL0 — SVCs only; kernel owns the NIC.
- No TCP/UDP, sockets, DHCP, DNS, or Wi‑Fi.
- No argv, environ, or libc.
- Does not replace `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, or `/udpdemo` — all six slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
