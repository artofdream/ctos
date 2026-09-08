#!/bin/sh
# cargo runner for aarch64-ctos. virt only — not a board claim.
# -semihosting lets tests/panics stop QEMU with a host exit code
# (ARM SYS_EXIT). The hello kernel does not call it.
exec qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -display none \
    -serial stdio \
    -semihosting \
    -kernel "$1"
