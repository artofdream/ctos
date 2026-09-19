#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
# -semihosting lets tests/panics stop QEMU with a host exit code
# (ARM SYS_EXIT). The hello kernel does not call it.
# -m 128M matches the M7 frame pool (ADR-008). virt default is 128M;
# pin it so a host `-m` habit cannot shrink RAM under the allocator.
# A7: attach a host-visible FAT16 image on virtio-mmio blk (ADR-028).
# Host `-drive` without guest virtio code is not a probe.
# N1 / ADR-066 + N2 / ADR-067 + N3 / ADR-070: QEMU user netdev + virtio-net-device.
# N5 / ADR-074: guestfwd TCP echo at 10.0.2.4:7 → scripts/tcp-echo-stdio.sh.
# Host `-netdev` without guest virtio-net code is not a probe.
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
APP5="${CTOS_APP5_ELF:-$ROOT/target/net-libctos.elf}"
APP6="${CTOS_APP6_ELF:-$ROOT/target/udp-libctos.elf}"
APP7="${CTOS_APP7_ELF:-$ROOT/target/mkdir-libctos.elf}"
APP8="${CTOS_APP8_ELF:-$ROOT/target/tcp-libctos.elf}"
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
if [ ! -f "$APP5" ]; then
    echo "qemu-aarch64: missing app5 ELF $APP5 (ADR-068; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP6" ]; then
    echo "qemu-aarch64: missing app6 ELF $APP6 (ADR-071; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP7" ]; then
    echo "qemu-aarch64: missing app7 ELF $APP7 (ADR-075; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$APP8" ]; then
    echo "qemu-aarch64: missing app8 ELF $APP8 (ADR-076; cargo build publishes it)" >&2
    exit 1
fi
if ! python3 "$ROOT/scripts/mkfat16.py" --app "$APP" --app2 "$APP2" --app3 "$APP3" --app4 "$APP4" --app5 "$APP5" --app6 "$APP6" --app7 "$APP7" --app8 "$APP8" "$IMG" >/dev/null; then
    echo "qemu-aarch64: failed to write FAT16 image $IMG" >&2
    exit 1
fi
# ADR-079: default probe CPU is cortex-a76 (FEAT_PAN; pan: present).
# Historical cortex-a57 stays pan: absent (ADR-026/054). Do not silent -cpu.
exec qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a76 \
    -m 128M \
    -display none \
    -serial stdio \
    -semihosting \
    -drive if=none,file="$IMG",format=raw,cache=writethrough,id=hd0 \
    -device virtio-blk-device,drive=hd0 \
    -netdev user,id=net0,guestfwd=tcp:10.0.2.4:7-cmd:"$ROOT/scripts/tcp-echo-stdio.sh" \
    -device virtio-net-device,netdev=net0 \
    -kernel "$1"
