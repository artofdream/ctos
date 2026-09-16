#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
# -semihosting lets tests/panics stop QEMU with a host exit code
# (ARM SYS_EXIT). The hello kernel does not call it.
# -m 128M matches the M7 frame pool (ADR-008). virt default is 128M;
# pin it so a host `-m` habit cannot shrink RAM under the allocator.
# A7: attach a host-visible FAT16 image on virtio-mmio blk (ADR-028).
# Host `-drive` without guest virtio code is not a probe.
# N1 / ADR-066 + N2 / ADR-067: QEMU user netdev + virtio-net-device (mmio). Host
# `-netdev` without guest virtio-net code is not a probe.
# ADR-045 taken-SError QMP lives in qemu-serial-inject.py (hello smoke),
# not this cargo-test runner — no #[test_case] without a host inject.
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IMG="${CTOS_BLK_IMAGE:-$ROOT/target/fat16.img}"
if ! command -v python3 >/dev/null 2>&1; then
    echo "qemu-aarch64: python3 needed to build the FAT16 image" >&2
    exit 1
fi
APP="${CTOS_APP_ELF:-$ROOT/target/hello-libctos.elf}"
APP2="${CTOS_APP2_ELF:-$ROOT/target/fs-libctos.elf}"
APP3="${CTOS_APP3_ELF:-$ROOT/target/fat-libctos.elf}"
APP4="${CTOS_APP4_ELF:-$ROOT/target/yield-libctos.elf}"
if [ ! -f "$APP" ]; then
    echo "qemu-aarch64: missing app ELF $APP (A9 / ADR-030; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP2" ]; then
    echo "qemu-aarch64: missing app2 ELF $APP2 (ADR-059; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP3" ]; then
    echo "qemu-aarch64: missing app3 ELF $APP3 (ADR-061; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP4" ]; then
    echo "qemu-aarch64: missing app4 ELF $APP4 (ADR-062; cargo build publishes it)" >&2
    exit 1
fi
if ! python3 "$ROOT/scripts/mkfat16.py" --app "$APP" --app2 "$APP2" --app3 "$APP3" --app4 "$APP4" "$IMG" >/dev/null; then
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
    -netdev user,id=net0 \
    -device virtio-net-device,netdev=net0 \
    -kernel "$1"
