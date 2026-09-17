# `tcp-libctos` — freestanding EL0 TCP-echo SVC sample

Eighth freestanding `no_std` sample linked against [`libctos`](../../libctos). Catalog expansion ([ADR-076](../../docs/03-adr/ADR-076-el0-tcp-svc.md)). Complements [`hello-libctos`](../hello-libctos/README.md), [`fs-libctos`](../fs-libctos/README.md), [`fat-libctos`](../fat-libctos/README.md), [`yield-libctos`](../yield-libctos/README.md), [`net-libctos`](../net-libctos/README.md), [`udp-libctos`](../udp-libctos/README.md), and [`mkdir-libctos`](../mkdir-libctos/README.md).

This is **not** a hosted app, not glibc, not POSIX, not a TCP product stack, not BSD sockets, not listen/accept, not TLS, not DHCP/DNS as product, not “has networking,” and not a guest virtio-net driver in EL0.

## What it prints

`main` writes `libctos: tcp-hi`, asks the kernel to run the thin TCP echo probe vs QEMU `guestfwd` `10.0.2.4:7` (`net_tcp_echo`), then `libctos: tcp-ok` and returns 0. Smoke also greps kernel `tcpdemo: ok` from the FAT `/tcpdemo` load path. Kernel `net: tcp-ok` (ADR-074) stays a separate N5 marker.

## Rebuild

The kernel `build.rs` builds this crate and publishes `target/tcp-libctos.elf`. `scripts/mkfat16.py` stores it as FAT `/tcpdemo` beside the prior seven samples. Usual guest rebuild:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses):

```bash
cargo build --release \
  --manifest-path user/tcp-libctos/Cargo.toml \
  --target user/tcp-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No virtio-net programming from EL0 — SVCs only; kernel owns the NIC.
- No BSD sockets, listen/accept, multi-connection table, TLS, HTTP, DHCP, DNS product, or Wi‑Fi.
- No argv, environ, or libc.
- Does not replace prior samples — all eight slots stay on the image.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
