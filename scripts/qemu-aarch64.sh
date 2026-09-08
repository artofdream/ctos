#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
exec qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -nographic \
    -serial mon:stdio \
    -kernel "$1"
