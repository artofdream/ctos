#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
# -semihosting lets tests/panics stop QEMU with a host exit code
# (ARM SYS_EXIT). The hello kernel does not call it.
# -m 128M matches the M7 frame pool (ADR-008). virt default is 128M;
# pin it so a host `-m` habit cannot shrink RAM under the allocator.
exec qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -m 128M \
    -display none \
    -serial stdio \
    -semihosting \
    -kernel "$1"
