#!/bin/sh
# Fail-closed host smoke: build, require UART hello + BRK + fatal nested
# lines, then cargo test. Used by Docker and GitHub Actions. Do not
# treat file presence as boot.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

HELLO="${CTOS_HELLO_STRING:-Hello World!}"
BRK="${CTOS_BRK_STRING:-exception: sync BRK}"
FATAL="${CTOS_FATAL_STRING:-exception: fatal nested}"
TIMEOUT_SECS="${CTOS_QEMU_TIMEOUT:-8}"

if ! command -v qemu-system-aarch64 >/dev/null 2>&1; then
    echo "qemu-smoke: qemu-system-aarch64 not on PATH" >&2
    exit 1
fi

echo "qemu-smoke: cargo build"
cargo +nightly build

elf="target/aarch64-ctos/debug/ctos"
if [ ! -x "$elf" ]; then
    echo "qemu-smoke: missing $elf" >&2
    exit 1
fi

log=$(mktemp)
trap 'rm -f "$log"' EXIT

echo "qemu-smoke: QEMU virt serial (timeout ${TIMEOUT_SECS}s)"
set +e
timeout "$TIMEOUT_SECS" qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a57 \
    -display none \
    -serial stdio \
    -semihosting \
    -kernel "$elf" >"$log" 2>&1
qemu_ec=$?
set -e

cat "$log"
# 124 = timeout(1) killed the looping hello kernel. That is expected.
# Any other hang/crash without the string is a fail.
if ! grep -q "$HELLO" "$log"; then
    echo "qemu-smoke: missing '$HELLO' on serial (qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: hello string present (qemu exit $qemu_ec)"
if ! grep -q "$BRK" "$log"; then
    echo "qemu-smoke: missing '$BRK' on serial (VBAR/BRK path, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: BRK handler string present"
if ! grep -q "$FATAL" "$log"; then
    echo "qemu-smoke: missing '$FATAL' on serial (FR-07 fatal path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "exception: fatal probe missed" "$log"; then
    echo "qemu-smoke: fatal probe missed (nested path did not run)" >&2
    exit 1
fi
echo "qemu-smoke: fatal nested string present"

echo "qemu-smoke: cargo test (semihosting exit)"
# The cargo runner is scripts/qemu-aarch64.sh. Tests must exit themselves.
# Belt: do not let a missed SYS_EXIT hang CI.
set +e
timeout 30 cargo +nightly test -- --nocapture
test_ec=$?
set -e
if [ "$test_ec" -ne 0 ]; then
    echo "qemu-smoke: cargo test failed (exit $test_ec)" >&2
    exit "$test_ec"
fi

echo "qemu-smoke: force-fail must be non-zero"
set +e
timeout 30 cargo +nightly test --features force-fail -- --nocapture
fail_ec=$?
set -e
if [ "$fail_ec" -eq 0 ]; then
    echo "qemu-smoke: force-fail unexpectedly passed" >&2
    exit 1
fi
if [ "$fail_ec" -eq 124 ]; then
    echo "qemu-smoke: force-fail timed out (semihosting exit missing?)" >&2
    exit 1
fi
echo "qemu-smoke: force-fail exited $fail_ec (fail-closed ok)"

echo "qemu-smoke: ok"
