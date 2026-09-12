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
python3 "$ROOT/scripts/mkfat16.py" "$IMG" >/dev/null
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
