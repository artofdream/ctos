#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
# -semihosting lets tests/panics stop QEMU with a host exit code
# (ARM SYS_EXIT). The hello kernel does not call it.
# -m 128M matches the M7 frame pool (ADR-008). virt default is 128M;
# pin it so a host `-m` habit cannot shrink RAM under the allocator.
# A7: attach a host-visible FAT16 image on virtio-mmio blk (ADR-028).
# Host `-drive` without guest virtio code is not a probe.
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IMG="${CTOS_BLK_IMAGE:-$ROOT/target/fat16.img}"
if ! command -v python3 >/dev/null 2>&1; then
    echo "qemu-aarch64: python3 needed to build the FAT16 image" >&2
    exit 1
fi
APP="${CTOS_APP_ELF:-$ROOT/target/hello-libctos.elf}"
if [ ! -f "$APP" ]; then
    echo "qemu-aarch64: missing app ELF $APP (A9 / ADR-030; cargo build publishes it)" >&2
    exit 1
fi
if ! python3 "$ROOT/scripts/mkfat16.py" --app "$APP" "$IMG" >/dev/null; then
    echo "qemu-aarch64: failed to write FAT16 image $IMG" >&2
    exit 1
fi
exec qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -m 128M \
    -display none \
    -serial stdio \
    -semihosting \
    -drive if=none,file="$IMG",format=raw,id=hd0 \
    -device virtio-blk-device,drive=hd0 \
    -kernel "$1"
