#!/bin/sh
# Fail-closed host smoke: build, require UART hello + paging + heap +
# two-task sched + W^X + stack guards + RO+NX text/data + EL0 first mile +
# EL0 no-kernel-read + standing EL0 + ASID isolation + TTBR1 private page +
# TTBR1 high-VA EL1 exec + CNTPCT baseline + boot-delta + IRQ-to-handler
# delta + host ELF size +
# timer tick + injected UART RX + BRK + fatal nested lines, then cargo test.
# Used by Docker and GitHub Actions. Do not treat file presence as boot.
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

HELLO="${CTOS_HELLO_STRING:-Hello World!}"
PAGING="${CTOS_PAGING_STRING:-paging: ok}"
HEAP="${CTOS_HEAP_STRING:-heap: ok}"
SCHED="${CTOS_SCHED_STRING:-sched: ok}"
SCHED_A="${CTOS_SCHED_A_STRING:-sched: task a}"
SCHED_B="${CTOS_SCHED_B_STRING:-sched: task b}"
PERF="${CTOS_PERF_STRING:-perf: cntpct}"
IRQDELTA="${CTOS_IRQDELTA_STRING:-perf: irq-delta}"
WX="${CTOS_WX_STRING:-wx: ok}"
GUARD="${CTOS_GUARD_STRING:-guard: ok}"
RO="${CTOS_RO_STRING:-ro: ok}"
EL0="${CTOS_EL0_STRING:-el0: ok}"
ASID="${CTOS_ASID_STRING:-asid: ok}"
TTBR1="${CTOS_TTBR1_STRING:-ttbr1: ok}"
BOOTDELTA="${CTOS_BOOTDELTA_STRING:-perf: boot-delta}"
ELFSIZE="${CTOS_ELFSIZE_STRING:-perf: elf-size}"
TICK="${CTOS_TICK_STRING:-timer: tick}"
INPUT="${CTOS_INPUT_STRING:-input: rx 0x41}"
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

# NFR-08 host probe: debug ELF byte size. Not a budget, not a bench.
elf_bytes=$(wc -c < "$elf" | tr -d ' ')
echo "$ELFSIZE bytes=$elf_bytes"
if [ -z "$elf_bytes" ] || [ "$elf_bytes" -lt 4096 ]; then
    echo "qemu-smoke: elf-size implausibly small ($elf_bytes)" >&2
    exit 1
fi
echo "qemu-smoke: host ELF size present ($elf_bytes bytes)"

log=$(mktemp)
trap 'rm -f "$log"' EXIT

if ! command -v python3 >/dev/null 2>&1; then
    echo "qemu-smoke: python3 not on PATH (needed to inject UART RX)" >&2
    exit 1
fi

echo "qemu-smoke: QEMU virt serial + RX inject (timeout ${TIMEOUT_SECS}s)"
set +e
CTOS_QEMU_TIMEOUT="$TIMEOUT_SECS" python3 "$ROOT/scripts/qemu-serial-inject.py" \
    "$elf" >"$log" 2>&1
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
if ! grep -q "$PAGING" "$log"; then
    echo "qemu-smoke: missing '$PAGING' on serial (FR-09 paging path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "paging: probe missed" "$log"; then
    echo "qemu-smoke: paging probe missed (MMU / map-unmap path did not run)" >&2
    exit 1
fi
echo "qemu-smoke: paging string present"
if ! grep -q "$HEAP" "$log"; then
    echo "qemu-smoke: missing '$HEAP' on serial (FR-10 heap path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "heap: probe missed" "$log"; then
    echo "qemu-smoke: heap probe missed (Box/Vec path did not run)" >&2
    exit 1
fi
echo "qemu-smoke: heap string present"
if ! grep -q "$SCHED_A" "$log"; then
    echo "qemu-smoke: missing '$SCHED_A' on serial (FR-11 task a, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "$SCHED_B" "$log"; then
    echo "qemu-smoke: missing '$SCHED_B' on serial (FR-11 task b, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "$SCHED" "$log"; then
    echo "qemu-smoke: missing '$SCHED' on serial (FR-11 scheduler path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "sched: probe missed" "$log"; then
    echo "qemu-smoke: sched probe missed (two-task path did not run)" >&2
    exit 1
fi
echo "qemu-smoke: scheduler strings present"
if ! grep -q "$WX" "$log"; then
    echo "qemu-smoke: missing '$WX' on serial (NFR-10 W^X path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "wx: probe missed" "$log"; then
    echo "qemu-smoke: wx probe missed (heap PXN / execute-from-heap did not run)" >&2
    exit 1
fi
echo "qemu-smoke: W^X string present"
if ! grep -q "$GUARD" "$log"; then
    echo "qemu-smoke: missing '$GUARD' on serial (NFR-10 guard-page path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "guard: probe missed" "$log"; then
    echo "qemu-smoke: guard probe missed (linker-stack guard store did not fault)" >&2
    exit 1
fi
echo "qemu-smoke: stack-guard string present"
if ! grep -q "$RO" "$log"; then
    echo "qemu-smoke: missing '$RO' on serial (NFR-10 RO+NX path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ro: probe missed" "$log"; then
    echo "qemu-smoke: ro probe missed (execute-from-.data or write-to-RO-text did not run)" >&2
    exit 1
fi
if ! grep -q "ro: nx data" "$log"; then
    echo "qemu-smoke: missing 'ro: nx data' on serial (execute-from-.data, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ro: write fault" "$log"; then
    echo "qemu-smoke: missing 'ro: write fault' on serial (write-to-RO-text, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: RO+NX string present"
if ! grep -q "$EL0" "$log"; then
    echo "qemu-smoke: missing '$EL0' on serial (NFR-10 EL0 first-mile path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "el0: probe missed" "$log"; then
    echo "qemu-smoke: el0 probe missed (SVC round-trip or UXN IABORT did not run)" >&2
    exit 1
fi
if ! grep -q "el0: svc" "$log"; then
    echo "qemu-smoke: missing 'el0: svc' on serial (lower-EL SVC slot, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: nx kernel" "$log"; then
    echo "qemu-smoke: missing 'el0: nx kernel' on serial (EL0 UXN IABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: no kernel read" "$log"; then
    echo "qemu-smoke: missing 'el0: no kernel read' on serial (EL0 kernel-data load, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: standing" "$log"; then
    echo "qemu-smoke: missing 'el0: standing' on serial (standing EL0 context, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: restored" "$log"; then
    echo "qemu-smoke: missing 'el0: restored' on serial (standing EL0 teardown, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: EL0 first-mile + read-mile + standing strings present"
if ! grep -q "$ASID" "$log"; then
    echo "qemu-smoke: missing '$ASID' on serial (NFR-10 ASID isolation mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "asid: probe missed" "$log"; then
    echo "qemu-smoke: asid probe missed (dual ASID / conflict path did not run)" >&2
    exit 1
fi
if grep -q "asid: stale" "$log"; then
    echo "qemu-smoke: asid stale TLB entry used across ASIDs (isolation Failed)" >&2
    exit 1
fi
if ! grep -q "asid: dual" "$log"; then
    echo "qemu-smoke: missing 'asid: dual' on serial (ASID 1 vs 2 data, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "asid: conflict" "$log"; then
    echo "qemu-smoke: missing 'asid: conflict' on serial (stale-entry fault, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: ASID isolation strings present"
if ! grep -q "$TTBR1" "$log"; then
    echo "qemu-smoke: missing '$TTBR1' on serial (NFR-10 TTBR1 private-page mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ttbr1: probe missed" "$log"; then
    echo "qemu-smoke: ttbr1 probe missed (high-page EL1/EL0 path did not run)" >&2
    exit 1
fi
if grep -q "ttbr1: leaked" "$log"; then
    echo "qemu-smoke: ttbr1 leaked to EL0 (private page was visible)" >&2
    exit 1
fi
if ! grep -q "ttbr1: el1" "$log"; then
    echo "qemu-smoke: missing 'ttbr1: el1' on serial (EL1 high-page access, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ttbr1: no el0" "$log"; then
    echo "qemu-smoke: missing 'ttbr1: no el0' on serial (EL0 high-page DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ttbr1: exec missed" "$log"; then
    echo "qemu-smoke: ttbr1 exec missed (EL1 high-VA fetch path did not run)" >&2
    exit 1
fi
if ! grep -q "ttbr1: el1 exec" "$log"; then
    echo "qemu-smoke: missing 'ttbr1: el1 exec' on serial (EL1 TTBR1 fetch, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ttbr1: vbar" "$log"; then
    echo "qemu-smoke: missing 'ttbr1: vbar' on serial (VBAR high alias, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: TTBR1 private-page + high-VA exec strings present"
if ! grep -q "$PERF" "$log"; then
    echo "qemu-smoke: missing '$PERF' on serial (NFR-07 CNTPCT path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: probe missed" "$log"; then
    echo "qemu-smoke: perf probe missed (CNTPCT loop did not advance)" >&2
    exit 1
fi
echo "qemu-smoke: CNTPCT perf string present"
if ! grep -q "$BOOTDELTA" "$log"; then
    echo "qemu-smoke: missing '$BOOTDELTA' on serial (NFR-08 boot-delta path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: boot-delta missed" "$log"; then
    echo "qemu-smoke: boot-delta probe missed (CNTPCT did not advance from early to ready)" >&2
    exit 1
fi
echo "qemu-smoke: boot-delta perf string present"
if ! grep -q "$IRQDELTA" "$log"; then
    echo "qemu-smoke: missing '$IRQDELTA' on serial (NFR-07 IRQ-delta path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: irq-delta missed" "$log"; then
    echo "qemu-smoke: irq-delta probe missed (no CNTPCT−CVAL samples)" >&2
    exit 1
fi
echo "qemu-smoke: IRQ-delta perf string present"
if ! grep -q "$TICK" "$log"; then
    echo "qemu-smoke: missing '$TICK' on serial (FR-08 timer path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "timer: tick missed" "$log"; then
    echo "qemu-smoke: timer tick missed (CNTP/GIC path did not run)" >&2
    exit 1
fi
echo "qemu-smoke: timer tick string present"
if ! grep -q "$INPUT" "$log"; then
    echo "qemu-smoke: missing '$INPUT' on serial (FR-08 UART RX path, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "input: rx missed" "$log"; then
    echo "qemu-smoke: UART RX missed (injected byte did not arrive)" >&2
    exit 1
fi
if grep -q "input: rx .*unexpected" "$log"; then
    echo "qemu-smoke: UART RX byte was not the injected probe" >&2
    exit 1
fi
echo "qemu-smoke: UART RX string present"
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
