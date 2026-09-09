# Session memory — 2026-09-09 (Docker CRLF + cc + ROM)

cts-ai linux/arm64 `docker build` succeeded. `docker run` failed: (1) `#!/bin/sh\r` (2) `linker cc not found` on `compiler_builtins` (3) after `cargo build`, `failed to find romfile "efi-virtio.rom"` (`--no-install-recommends` skipped `ipxe-qemu`). File lives at `/usr/lib/ipxe/qemu/efi-virtio.rom` (`dpkg -L ipxe-qemu`). Ratchets: `.gitattributes` eol=lf; `build-essential`; `qemu-efi-aarch64` + `ipxe-qemu` + image `test -e` that ROM. Full smoke still unprobed here.
