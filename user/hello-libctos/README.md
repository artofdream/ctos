# `hello-libctos` — standing EL0 / loader recipe

Freestanding `no_std` hello linked against [`libctos`](../../libctos). Track A miles A2–A4 ([ADR-022](../../docs/03-adr/ADR-022-libctos-crt.md), [ADR-023](../../docs/03-adr/ADR-023-elf-pt-load-loader.md), [ADR-024](../../docs/03-adr/ADR-024-standing-el0-normal.md)).

This is **not** a hosted app, not glibc, not `exec` of a file on disk.

## What it prints

`main` writes `libctos: hi`, yields, writes `libctos: ok`, returns 0. The CRT (`libctos/src/crt0.S`) then `exit`s. Smoke also greps `libctos: linked`, `loader: mapped` / `loader: ok`, and `el0: task-ok` from the kernel path that embeds and loads this ELF.

## Rebuild

The kernel `build.rs` already builds this crate and publishes `target/hello-libctos.elf`. A2–A4 and A9 read FAT `/hello`. The usual guest rebuild is enough:

```bash
cargo build                 # from the repo root
./scripts/qemu-smoke.sh
```

Standalone (same target the kernel uses; publish + FAT `/hello` to run on the guest):

```bash
cargo build --release \
  --manifest-path user/hello-libctos/Cargo.toml \
  --target user/hello-libctos/aarch64-ctos-user.json
```

| Piece | Why |
| --- | --- |
| `aarch64-ctos-user.json` | Separate target **name** so parent kernel `rustflags` / `linker.ld` do not apply |
| `linker.ld` | Load VA `0x80002000` (`paging::EL0_PAGE`) |
| `.cargo/config.toml` | `build-std` for `core` only |

Do not use `aarch64-unknown-linux-gnu`. A Linux `ET_DYN` will not load.

## What this payload does not do

- No `fs_open` / `fs_read`. `libctos` exports those wrappers; this hello does not call them.
- No argv, environ, or libc.
- A2–A4 and A9 load `target/hello-libctos.elf` from FAT `/hello` (`libctos:*` / `loader:*` / `el0: task-*` / `slot: ok`). Cross-update is the leftover two-boot smoke on this OS and `ba6541c`.

Catalog: [user/README.md](../README.md). Site: [What can run today](../../docs/overview/what-can-run.md).
