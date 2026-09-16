# `udp-libctos` — freestanding EL0 UDP-DNS SVC sample

Sixth freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-071](../../docs/03-adr/ADR-071-n3x-el0-udp-svc.md)). Complements [`hello-libctos`](../hello-libctos/README.md), [`fs-libctos`](../fs-libctos/README.md), [`fat-libctos`](../fat-libctos/README.md), [`yield-libctos`](../yield-libctos/README.md), and [`net-libctos`](../net-libctos/README.md).

This is **not** a hosted app, not glibc, not POSIX, not a TCP stack, not BSD sockets, not DHCP/DNS as product, not “has networking,” and not a guest virtio-net driver in EL0.

## What it prints

`main` writes `libctos: udp-hi`, asks the kernel to UDP-DNS-probe SLIRP `10.0.2.3:53` (`net_udp_dns`), then `libctos: udp-ok` and returns 0. Smoke also greps kernel `udpdemo: ok` from the FAT `/udpdemo` load path.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/udp-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/udpdemo` beside `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, and `/netdemo`. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/udp-libctos/Cargo.toml \
  --target user/udp-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No virtio-net programming from EL0 — SVCs only; kernel owns the NIC.
- No BSD sockets, TCP product, generic UDP send/recv buffers, DHCP, DNS product, or Wi‑Fi.
- No argv, environ, or libc.
- Does not replace `/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`, or `/netdemo` — all six slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
