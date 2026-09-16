#!/bin/sh
# Fail-closed host smoke: build, require UART hello + paging + heap +
# two-task sched + W^X + stack guards + RO+NX text/data + EL0 first mile +
# EL0 no-kernel-read + standing EL0 + lower-EL IRQ while standing (ADR-040) + lower-EL FIQ (ADR-043) + SError QMP attempt (ADR-045) + SVC ABI (ADR-021) + libctos CRT (ADR-022) + guest ELF loader (ADR-023) + standing task (ADR-024) + ASID isolation + TTBR1 private page +
# TTBR1 high-VA EL1 exec + identity-tear (ADR-018/019/020/025/037/038/049) + PAN ID (ADR-026) + CNTPCT baseline + boot-delta + IRQ-to-handler
# delta + host ELF size +
# timer tick + injected UART RX + BRK + fatal nested lines, then cargo test.
# virtio-blk + FAT16 (ADR-028): host builds target/fat16.img and QEMU
# attaches `-drive if=none,file=...,id=hd0 -device virtio-blk-device,drive=hd0`.
# A9 / ADR-030: the same image also carries FAT /hello (app ELF).
# Leftover mile (ADR-032): A2–A4 also load that FAT file (no embed);
# host smoke then boots the same app ELF on documented prior OS
# ba6541c (A9 merge — earliest main tip with the slot path).
# Host `-drive` without guest virtio + VFS read is not a probe.
# N1 / ADR-066: QEMU also attaches `-netdev user,id=net0`
# `-device virtio-net-device,netdev=net0`. Host netdev alone is not a probe.
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
IDENT="${CTOS_IDENT_STRING:-ident: ok}"
BOOTDELTA="${CTOS_BOOTDELTA_STRING:-perf: boot-delta}"
ELFSIZE="${CTOS_ELFSIZE_STRING:-perf: elf-size}"
TICK="${CTOS_TICK_STRING:-timer: tick}"
INPUT="${CTOS_INPUT_STRING:-input: rx 0x41}"
BRK="${CTOS_BRK_STRING:-exception: sync BRK}"
FATAL="${CTOS_FATAL_STRING:-exception: fatal nested}"
TIMEOUT_SECS="${CTOS_QEMU_TIMEOUT:-8}"

# NFR-03: Linux coreutils has sha256sum; macOS typically has shasum, not
# sha256sum. OpenSSL is a third option. The same-app proof is still the
# `cp` of the published ELF; this only pins the bytes in the log.
sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$1" | awk '{print $NF}'
    else
        return 1
    fi
}

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

# A9 host artifacts: OS image (already $elf) + published app ELF.
# ADR-059: second freestanding sample ELF for FAT /fsdemo.
# ADR-061: third freestanding sample ELF for FAT /fatdemo.
# ADR-062: fourth freestanding sample ELF for FAT /yldemo.
app="${CTOS_APP_ELF:-$ROOT/target/hello-libctos.elf}"
app2="${CTOS_APP2_ELF:-$ROOT/target/fs-libctos.elf}"
app3="${CTOS_APP3_ELF:-$ROOT/target/fat-libctos.elf}"
app4="${CTOS_APP4_ELF:-$ROOT/target/yield-libctos.elf}"
if [ ! -f "$app" ]; then
    echo "qemu-smoke: missing app payload $app (A9 / ADR-030; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$app2" ]; then
    echo "qemu-smoke: missing app2 payload $app2 (ADR-059; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$app3" ]; then
    echo "qemu-smoke: missing app3 payload $app3 (ADR-061; cargo build publishes it)" >&2
    exit 1
fi
if [ ! -f "$app4" ]; then
    echo "qemu-smoke: missing app4 payload $app4 (ADR-062; cargo build publishes it)" >&2
    exit 1
fi
app_bytes=$(wc -c < "$app" | tr -d ' ')
app2_bytes=$(wc -c < "$app2" | tr -d ' ')
app3_bytes=$(wc -c < "$app3" | tr -d ' ')
app4_bytes=$(wc -c < "$app4" | tr -d ' ')
if [ -z "$app_bytes" ] || [ "$app_bytes" -lt 64 ]; then
    echo "qemu-smoke: app ELF implausibly small ($app_bytes)" >&2
    exit 1
fi
if [ -z "$app2_bytes" ] || [ "$app2_bytes" -lt 64 ]; then
    echo "qemu-smoke: app2 ELF implausibly small ($app2_bytes)" >&2
    exit 1
fi
if [ -z "$app3_bytes" ] || [ "$app3_bytes" -lt 64 ]; then
    echo "qemu-smoke: app3 ELF implausibly small ($app3_bytes)" >&2
    exit 1
fi
if [ -z "$app4_bytes" ] || [ "$app4_bytes" -lt 64 ]; then
    echo "qemu-smoke: app4 ELF implausibly small ($app4_bytes)" >&2
    exit 1
fi
if cmp -s "$elf" "$app"; then
    echo "qemu-smoke: OS image and app payload are the same file" >&2
    exit 1
fi
if cmp -s "$app" "$app2"; then
    echo "qemu-smoke: hello and fsdemo payloads are the same file" >&2
    exit 1
fi
if cmp -s "$app" "$app3" || cmp -s "$app2" "$app3"; then
    echo "qemu-smoke: fatdemo payload duplicates hello or fsdemo" >&2
    exit 1
fi
if cmp -s "$app" "$app4" || cmp -s "$app2" "$app4" || cmp -s "$app3" "$app4"; then
    echo "qemu-smoke: yldemo payload duplicates hello, fsdemo, or fatdemo" >&2
    exit 1
fi
echo "qemu-smoke: OS image $elf ($elf_bytes bytes) + app payload $app ($app_bytes bytes) + app2 $app2 ($app2_bytes bytes) + app3 $app3 ($app3_bytes bytes) + app4 $app4 ($app4_bytes bytes)"
echo "qemu-smoke: kernel rebuild compiled app payload (build.rs published $app)"
echo "qemu-smoke: kernel rebuild compiled fsdemo payload (build.rs published $app2)"
echo "qemu-smoke: kernel rebuild compiled fatdemo payload (build.rs published $app3)"
echo "qemu-smoke: kernel rebuild compiled yldemo payload (build.rs published $app4)"

# ADR-039: standing/EL0 trampoline path must not TLBI VMALLE1.
if grep -n 'tlbi vmalle1' src/exception.rs; then
    echo "qemu-smoke: src/exception.rs still has tlbi vmalle1 (ADR-039)" >&2
    exit 1
fi
echo "qemu-smoke: exception.rs has no tlbi vmalle1"

# Embed honesty: A2–A4 must prove markers via FAT /hello, not include_bytes!.
# ADR-046 allows probe-only include_bytes! in src/slot.rs for CNTPCT compare.
if grep -n 'include_bytes!.*hello-libctos' src/libctos.rs src/loader.rs 2>/dev/null; then
    echo "qemu-smoke: A2/A3 still embeds hello-libctos (must load FAT /hello)" >&2
    exit 1
fi
if ! grep -n 'include_bytes!.*hello-libctos' src/slot.rs >/dev/null; then
    echo "qemu-smoke: missing ADR-046 probe-only include_bytes! in src/slot.rs" >&2
    exit 1
fi
echo "qemu-smoke: A2/A3 do not include_bytes! hello-libctos (slot.rs probe-only OK)"

# A7 host-visible FAT16 (ADR-028) + A9 /hello app slot.
img="${CTOS_BLK_IMAGE:-$ROOT/target/fat16.img}"
python3 "$ROOT/scripts/mkfat16.py" --app "$app" --app2 "$app2" --app3 "$app3" --app4 "$app4" "$img"
python3 "$ROOT/scripts/mkfat16.py" --check --require-app --require-app2 --require-app3 --require-app4 "$img"
export CTOS_BLK_IMAGE="$img"
export CTOS_APP_ELF="$app"
export CTOS_APP2_ELF="$app2"
export CTOS_APP3_ELF="$app3"
export CTOS_APP4_ELF="$app4"
echo "qemu-smoke: FAT16 image $img (host-visible; not a guest probe)"

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
if ! grep -q "el0: irq" "$log"; then
    echo "qemu-smoke: missing 'el0: irq' on serial (ADR-040 lower-EL IRQ while standing, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: irq-default" "$log"; then
    echo "qemu-smoke: missing 'el0: irq-default' on serial (ADR-041 IRQ-unmasked default ERET, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: fiq" "$log"; then
    echo "qemu-smoke: missing 'el0: fiq' on serial (ADR-043 lower-EL FIQ while standing, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: serror-park" "$log"; then
    echo "qemu-smoke: missing 'el0: serror-park' on serial (ADR-053 taken SError hard-stopped; park honesty, qemu exit $qemu_ec)" >&2
    exit 1
fi
# ADR-045: QMP inject-nmi is attempted by qemu-serial-inject.py. On QEMU 10
# virt+cortex-a57 it fails ("machine does not provide NMIs"). Do NOT require
# `el0: serror` here — that would fake Verified. Soft-note if the taken
# marker appears without the park suffix.
if grep -E -q 'el0: serror$' "$log"; then
    echo "qemu-smoke: note: taken 'el0: serror' present (would be Verified only with a working inject)"
fi
if grep -q "qemu-serial-inject: qmp inject-nmi" "$log"; then
    echo "qemu-smoke: QMP inject-nmi attempt logged (ADR-045)"
fi
if ! grep -q "el0: no-vmalle1" "$log"; then
    echo "qemu-smoke: missing 'el0: no-vmalle1' on serial (ADR-039 EL0 entry without VMALLE1, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: EL0 first-mile + read-mile + standing + irq + irq-default + fiq + serror-park + no-vmalle1 strings present (taken SError deferred/hard-stopped (ADR-053))"
if grep -q "svc: probe missed" "$log"; then
    echo "qemu-smoke: svc probe missed (EL0 ABI trip did not run)" >&2
    exit 1
fi
if ! grep -q "svc: yield" "$log"; then
    echo "qemu-smoke: missing 'svc: yield' on serial (SYS_YIELD, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "svc: user-hi" "$log"; then
    echo "qemu-smoke: missing 'svc: user-hi' on serial (SYS_UART_WRITE user buffer, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "svc: uart" "$log"; then
    echo "qemu-smoke: missing 'svc: uart' on serial (SYS_UART_WRITE dispatch, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "svc: exit" "$log"; then
    echo "qemu-smoke: missing 'svc: exit' on serial (SYS_EXIT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "svc: ok" "$log"; then
    echo "qemu-smoke: missing 'svc: ok' on serial (ADR-021 ABI mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: SVC ABI strings present"
if grep -q "libctos: probe missed" "$log"; then
    echo "qemu-smoke: libctos probe missed (linked hello trip did not run)" >&2
    exit 1
fi
if ! grep -q "libctos: hi" "$log"; then
    echo "qemu-smoke: missing 'libctos: hi' on serial (libctos uart_write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: ok" "$log"; then
    echo "qemu-smoke: missing 'libctos: ok' on serial (libctos uart_write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: linked" "$log"; then
    echo "qemu-smoke: missing 'libctos: linked' on serial (A2 CRT mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: libctos CRT strings present"
if grep -q "loader: probe missed" "$log"; then
    echo "qemu-smoke: loader probe missed (guest ELF PT_LOAD trip did not run)" >&2
    exit 1
fi
if ! grep -q "loader: mapped" "$log"; then
    echo "qemu-smoke: missing 'loader: mapped' on serial (guest PT_LOAD map, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "loader: ok" "$log"; then
    echo "qemu-smoke: missing 'loader: ok' on serial (A3 guest loader mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: guest ELF loader strings present"
if grep -q "el0: task missed" "$log"; then
    echo "qemu-smoke: standing-task probe missed (A4 loaded-app trip did not run)" >&2
    exit 1
fi
if ! grep -q "el0: task-enter" "$log"; then
    echo "qemu-smoke: missing 'el0: task-enter' on serial (A4 standing task enter, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: task-active" "$log"; then
    echo "qemu-smoke: missing 'el0: task-active' on serial (is_active while standing, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: task-exit" "$log"; then
    echo "qemu-smoke: missing 'el0: task-exit' on serial (SYS_EXIT from standing task, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: task-restored" "$log"; then
    echo "qemu-smoke: missing 'el0: task-restored' on serial (A4 exit restore, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: restore-fail" "$log"; then
    echo "qemu-smoke: missing 'el0: restore-fail' on serial (A4 fail-closed fault restore, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "el0: task-ok" "$log"; then
    echo "qemu-smoke: missing 'el0: task-ok' on serial (A4 standing-as-normal mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: standing-task (A4) strings present"
if grep -q "fs: probe missed" "$log"; then
    echo "qemu-smoke: fs probe missed (memfs create/read/write trip did not run)" >&2
    exit 1
fi
if ! grep -q "fs: create" "$log"; then
    echo "qemu-smoke: missing 'fs: create' on serial (kernel memfs create, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fs: write" "$log"; then
    echo "qemu-smoke: missing 'fs: write' on serial (kernel memfs write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fs: read" "$log"; then
    echo "qemu-smoke: missing 'fs: read' on serial (kernel memfs read-back, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fs: el0" "$log"; then
    echo "qemu-smoke: missing 'fs: el0' on serial (EL0 memfs SVC trip, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fs: ok" "$log"; then
    echo "qemu-smoke: missing 'fs: ok' on serial (A6 thin VFS + memfs mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "vfs: mount" "$log"; then
    echo "qemu-smoke: missing 'vfs: mount' on serial (ADR-058 prefix mounts, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "vfs: mounts" "$log"; then
    echo "qemu-smoke: missing 'vfs: mounts' on serial (ADR-058 mount table, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: thin VFS + memfs (A6) strings present"
if grep -q "blk: probe missed" "$log"; then
    echo "qemu-smoke: blk probe missed (virtio-mmio sector R/W did not run)" >&2
    exit 1
fi
if ! grep -q "blk: virtio" "$log"; then
    echo "qemu-smoke: missing 'blk: virtio' on serial (virtio-mmio discover, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "blk: cap" "$log"; then
    echo "qemu-smoke: missing 'blk: cap' on serial (virtio-blk capacity, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "blk: rw" "$log"; then
    echo "qemu-smoke: missing 'blk: rw' on serial (sector write/read, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "blk: ok" "$log"; then
    echo "qemu-smoke: missing 'blk: ok' on serial (A7 virtio-blk mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: virtio-blk (A7) strings present"
if grep -q "net: probe missed" "$log"; then
    echo "qemu-smoke: net probe missed (virtio-net discover/TX/RX did not run)" >&2
    exit 1
fi
if ! grep -q "net: virtio" "$log"; then
    echo "qemu-smoke: missing 'net: virtio' on serial (virtio-net discover, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "net: mac" "$log"; then
    echo "qemu-smoke: missing 'net: mac' on serial (virtio-net MAC config, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "net: tx" "$log"; then
    echo "qemu-smoke: missing 'net: tx' on serial (virtio-net TX ARP, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "net: rx" "$log"; then
    echo "qemu-smoke: missing 'net: rx' on serial (virtio-net RX ARP reply, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "net: ok" "$log"; then
    echo "qemu-smoke: missing 'net: ok' on serial (N1 virtio-net first frame, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: virtio-net (N1) strings present"
if grep -q "fat: probe missed" "$log"; then
    echo "qemu-smoke: fat probe missed (FAT16 VFS /probe read did not run)" >&2
    exit 1
fi
if ! grep -q "fat: mount" "$log"; then
    echo "qemu-smoke: missing 'fat: mount' on serial (FAT16 BPB, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: read" "$log"; then
    echo "qemu-smoke: missing 'fat: read' on serial (VFS open/read /probe, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: write" "$log"; then
    echo "qemu-smoke: missing 'fat: write' on serial (VFS write /probe, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: rewrite" "$log"; then
    echo "qemu-smoke: missing 'fat: rewrite' on serial (restore /probe fat-hi, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: create" "$log"; then
    echo "qemu-smoke: missing 'fat: create' on serial (FAT create+/fwr, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: readdir" "$log"; then
    echo "qemu-smoke: missing 'fat: readdir' on serial (FAT16 root listing, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: entries" "$log"; then
    echo "qemu-smoke: missing 'fat: entries' on serial (FAT16 readdir count, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: delete" "$log"; then
    echo "qemu-smoke: missing 'fat: delete' on serial (FAT16 unlink /fdel, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: grow" "$log"; then
    echo "qemu-smoke: missing 'fat: grow' on serial (FAT16 multi-cluster grow /fgrow, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fat: ok" "$log"; then
    echo "qemu-smoke: missing 'fat: ok' on serial (A7 FAT16 mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: FAT16 (A7) strings present"
echo "qemu-smoke: FAT16 readdir (ADR-056) strings present"
echo "qemu-smoke: FAT16 delete (ADR-057) strings present"
echo "qemu-smoke: FAT16 multi-cluster grow (ADR-064) strings present"
# ADR-065: FAT-vs-memfs read CNTPCT pair (raw ticks; not a bench / percent).
if grep -q "perf: fat-read missed" "$log"; then
    echo "qemu-smoke: fat-read probe missed (CNTPCT around FAT VFS read did not advance)" >&2
    exit 1
fi
if ! grep -q "perf: fat-read" "$log"; then
    echo "qemu-smoke: missing 'perf: fat-read' on serial (ADR-065 FAT read CNTPCT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: memfs-read missed" "$log"; then
    echo "qemu-smoke: memfs-read probe missed (CNTPCT around memfs read did not advance)" >&2
    exit 1
fi
if ! grep -q "perf: memfs-read" "$log"; then
    echo "qemu-smoke: missing 'perf: memfs-read' on serial (ADR-065 memfs read CNTPCT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "perf: fs-read-delta" "$log"; then
    echo "qemu-smoke: missing 'perf: fs-read-delta' on serial (ADR-065 compared pair, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -E -q 'perf: fs-read-delta.*(faster|slower|percent|%)' "$log"; then
    echo "qemu-smoke: fs-read-delta must not claim faster/slower/percent" >&2
    exit 1
fi
echo "qemu-smoke: ADR-065 FAT-vs-memfs read CNTPCT pair present"
# ADR-051: FAT-vs-memfs write CNTPCT pair (raw ticks; not a bench / percent).
if grep -q "perf: fat-write missed" "$log"; then
    echo "qemu-smoke: fat-write probe missed (CNTPCT around FAT VFS write did not advance)" >&2
    exit 1
fi
if ! grep -q "perf: fat-write" "$log"; then
    echo "qemu-smoke: missing 'perf: fat-write' on serial (ADR-051 FAT write CNTPCT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: memfs-write missed" "$log"; then
    echo "qemu-smoke: memfs-write probe missed (CNTPCT around memfs write did not advance)" >&2
    exit 1
fi
if ! grep -q "perf: memfs-write" "$log"; then
    echo "qemu-smoke: missing 'perf: memfs-write' on serial (ADR-051 memfs write CNTPCT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "perf: fs-write-delta" "$log"; then
    echo "qemu-smoke: missing 'perf: fs-write-delta' on serial (ADR-051 compared pair, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -E -q 'perf: fs-write-delta.*(faster|slower|percent|%)' "$log"; then
    echo "qemu-smoke: fs-write-delta must not claim faster/slower/percent" >&2
    exit 1
fi
echo "qemu-smoke: ADR-051 FAT-vs-memfs write CNTPCT pair present"
# Host-visible write: guest left /fwr = fat-nw on the raw image (ADR-050).
if ! python3 "$ROOT/scripts/mkfat16.py" --check-write "$img"; then
    echo "qemu-smoke: host FAT write check failed (guest /fwr not on image)" >&2
    exit 1
fi
echo "qemu-smoke: FAT16 write (ADR-050) host-check ok"
if grep -q "slot: probe missed" "$log"; then
    echo "qemu-smoke: slot probe missed (FAT /hello load path did not run)" >&2
    exit 1
fi
if grep -q "slot: embed" "$log"; then
    echo "qemu-smoke: slot used the A3 embed (A9 path must be FAT /hello)" >&2
    exit 1
fi
if ! grep -q "slot: fat" "$log"; then
    echo "qemu-smoke: missing 'slot: fat' on serial (A9 FAT /hello read, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "slot: mapped" "$log"; then
    echo "qemu-smoke: missing 'slot: mapped' on serial (A9 PT_LOAD map, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "slot: ok" "$log"; then
    echo "qemu-smoke: missing 'slot: ok' on serial (A9 OS/app slot first cut, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "fsdemo: probe missed" "$log"; then
    echo "qemu-smoke: fsdemo probe missed (FAT /fsdemo load path did not run)" >&2
    exit 1
fi
if ! grep -q "fsdemo: fat" "$log"; then
    echo "qemu-smoke: missing 'fsdemo: fat' on serial (ADR-059 FAT /fsdemo read, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fsdemo: mapped" "$log"; then
    echo "qemu-smoke: missing 'fsdemo: mapped' on serial (ADR-059 PT_LOAD map, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: fs-hi" "$log"; then
    echo "qemu-smoke: missing 'libctos: fs-hi' on serial (fs-libctos uart_write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: fs-ok" "$log"; then
    echo "qemu-smoke: missing 'libctos: fs-ok' on serial (fs-libctos VFS trip, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "libctos: fs-fail" "$log"; then
    echo "qemu-smoke: saw 'libctos: fs-fail' on serial (fs-libctos VFS trip failed, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fsdemo: ok" "$log"; then
    echo "qemu-smoke: missing 'fsdemo: ok' on serial (ADR-059 second sample, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: ADR-059 fs-libctos /fsdemo strings present"

if grep -q "fatdemo: probe missed" "$log"; then
    echo "qemu-smoke: fatdemo probe missed (FAT /fatdemo load path did not run)" >&2
    exit 1
fi
if ! grep -q "fatdemo: fat" "$log"; then
    echo "qemu-smoke: missing 'fatdemo: fat' on serial (ADR-061 FAT /fatdemo read, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fatdemo: mapped" "$log"; then
    echo "qemu-smoke: missing 'fatdemo: mapped' on serial (ADR-061 PT_LOAD map, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: fat-hi" "$log"; then
    echo "qemu-smoke: missing 'libctos: fat-hi' on serial (fat-libctos uart_write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: fat-ok" "$log"; then
    echo "qemu-smoke: missing 'libctos: fat-ok' on serial (fat-libctos FAT VFS trip, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: fat-grow" "$log"; then
    echo "qemu-smoke: missing 'libctos: fat-grow' on serial (fat-libctos multi-cluster write, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "libctos: fat-fail" "$log"; then
    echo "qemu-smoke: saw 'libctos: fat-fail' on serial (fat-libctos FAT VFS trip failed, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "fatdemo: ok" "$log"; then
    echo "qemu-smoke: missing 'fatdemo: ok' on serial (ADR-061 third sample, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: ADR-061 fat-libctos /fatdemo strings present"

if grep -q "yldemo: probe missed" "$log"; then
    echo "qemu-smoke: yldemo probe missed (FAT /yldemo load path did not run)" >&2
    exit 1
fi
if ! grep -q "yldemo: fat" "$log"; then
    echo "qemu-smoke: missing 'yldemo: fat' on serial (ADR-062 FAT /yldemo read, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "yldemo: mapped" "$log"; then
    echo "qemu-smoke: missing 'yldemo: mapped' on serial (ADR-062 PT_LOAD map, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: yld-hi" "$log"; then
    echo "qemu-smoke: missing 'libctos: yld-hi' on serial (yield-libctos uart_write, qemu exit $qemu_ec)" >&2
    exit 1
fi
# Several cooperative beats (more than hello's single yield).
beat_n=$(grep -c "libctos: beat" "$log" || true)
if [ "${beat_n:-0}" -lt 3 ]; then
    echo "qemu-smoke: expected >=3 'libctos: beat' on serial (got ${beat_n:-0}; ADR-062 yield rounds, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "libctos: yld-ok" "$log"; then
    echo "qemu-smoke: missing 'libctos: yld-ok' on serial (yield-libctos yield rounds, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "yldemo: ok" "$log"; then
    echo "qemu-smoke: missing 'yldemo: ok' on serial (ADR-062 fourth sample, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: ADR-062 yield-libctos /yldemo strings present"
if grep -q "perf: app-load missed" "$log"; then
    echo "qemu-smoke: app-load probe missed (CNTPCT around FAT load did not advance)" >&2
    exit 1
fi
if ! grep -q "perf: app-load" "$log"; then
    echo "qemu-smoke: missing 'perf: app-load' on serial (A9 load CNTPCT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "perf: embed-load missed" "$log"; then
    echo "qemu-smoke: embed-load probe missed (ADR-046 CNTPCT around linked-in trip)" >&2
    exit 1
fi
if ! grep -q "perf: embed-load" "$log"; then
    echo "qemu-smoke: missing 'perf: embed-load' on serial (ADR-046 probe-only, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "perf: slot-delta" "$log"; then
    echo "qemu-smoke: missing 'perf: slot-delta' on serial (ADR-046 compared pair, qemu exit $qemu_ec)" >&2
    exit 1
fi
# Honesty: pair line is ticks only — reject invented percent marketing.
if grep -E -q 'perf: slot-delta.*(faster|slower|percent|%)' "$log"; then
    echo "qemu-smoke: slot-delta must not claim faster/slower/percent" >&2
    exit 1
fi
echo "qemu-smoke: OS/app slot (A9) + ADR-046 slot-delta strings present"
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
if ! grep -q "$IDENT" "$log"; then
    echo "qemu-smoke: missing '$IDENT' on serial (NFR-10 identity-tear mile, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ident: probe missed" "$log"; then
    echo "qemu-smoke: ident probe missed (split / torn-page path did not run)" >&2
    exit 1
fi
if grep -q "ident: leaked" "$log"; then
    echo "qemu-smoke: ident leaked (torn identity page was still usable)" >&2
    exit 1
fi
if grep -q "ident: range missed" "$log"; then
    echo "qemu-smoke: ident range missed (dedicated identity text range stayed mapped)" >&2
    exit 1
fi
if grep -q "ident: reloc missed" "$log"; then
    echo "qemu-smoke: ident reloc missed (vtable / fn-pointer rewrite did not run)" >&2
    exit 1
fi
if grep -q "ident: live missed" "$log"; then
    echo "qemu-smoke: ident live missed (live identity .text after the stub stayed mapped)" >&2
    exit 1
fi
if grep -q "ident: ro-reloc missed" "$log"; then
    echo "qemu-smoke: ident ro-reloc missed (.rodata pointer rewrite did not run)" >&2
    exit 1
fi
if grep -q "ident: rodata missed" "$log"; then
    echo "qemu-smoke: ident rodata missed (identity .rodata stayed mapped)" >&2
    exit 1
fi
if ! grep -q "ident: jump" "$log"; then
    echo "qemu-smoke: missing 'ident: jump' on serial (high-VA continuation, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: reloc" "$log"; then
    echo "qemu-smoke: missing 'ident: reloc' on serial (high-VA vtable rewrite, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: range" "$log"; then
    echo "qemu-smoke: missing 'ident: range' on serial (identity .text range tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: live" "$log"; then
    echo "qemu-smoke: missing 'ident: live' on serial (live identity .text tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: ro-reloc" "$log"; then
    echo "qemu-smoke: missing 'ident: ro-reloc' on serial (identity .rodata pointer rewrite, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: rodata" "$log"; then
    echo "qemu-smoke: missing 'ident: rodata' on serial (identity .rodata tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: rodata-fault" "$log"; then
    echo "qemu-smoke: missing 'ident: rodata-fault' on serial (EL1 identity .rodata DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: rodata-high" "$log"; then
    echo "qemu-smoke: missing 'ident: rodata-high' on serial (EL1 high .rodata load, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: split" "$log"; then
    echo "qemu-smoke: missing 'ident: split' on serial (TTBR1 RAM clone, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: fault" "$log"; then
    echo "qemu-smoke: missing 'ident: fault' on serial (EL1 identity IABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: high" "$log"; then
    echo "qemu-smoke: missing 'ident: high' on serial (EL1 high twin after tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: text" "$log"; then
    echo "qemu-smoke: missing 'ident: text' on serial (EL1 high fetch of torn .text, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: no el0" "$log"; then
    echo "qemu-smoke: missing 'ident: no el0' on serial (EL0 torn-page DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ident: data-reloc missed" "$log"; then
    echo "qemu-smoke: ident data-reloc missed (.data pointer rewrite did not run)" >&2
    exit 1
fi
if grep -q "ident: sp-high missed" "$log"; then
    echo "qemu-smoke: ident sp-high missed (SP was not relocated to the high twin)" >&2
    exit 1
fi
if grep -q "ident: data missed" "$log"; then
    echo "qemu-smoke: ident data missed (identity .data/.bss/stacks stayed mapped)" >&2
    exit 1
fi
if grep -q "ident: heap-reloc missed" "$log"; then
    echo "qemu-smoke: ident heap-reloc missed (heap pointer rewrite did not run)" >&2
    exit 1
fi
if grep -q "ident: heap missed" "$log"; then
    echo "qemu-smoke: ident heap missed (identity heap stayed mapped)" >&2
    exit 1
fi
if grep -q "ident: miss heap-ready" "$log"; then
    echo "qemu-smoke: identity heap tear did not publish" >&2
    exit 1
fi
if grep -q "ident: heap-stay" "$log"; then
    echo "qemu-smoke: ident heap-stay still printed (heap should be torn)" >&2
    exit 1
fi
if ! grep -q "ident: data-reloc" "$log"; then
    echo "qemu-smoke: missing 'ident: data-reloc' on serial (identity .data pointer rewrite, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: data lo=" "$log"; then
    echo "qemu-smoke: missing 'ident: data' on serial (identity .data tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: data-fault" "$log"; then
    echo "qemu-smoke: missing 'ident: data-fault' on serial (EL1 identity .data DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: data-high" "$log"; then
    echo "qemu-smoke: missing 'ident: data-high' on serial (EL1 high .data load, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: heap-reloc" "$log"; then
    echo "qemu-smoke: missing 'ident: heap-reloc' on serial (identity heap pointer rewrite, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: heap lo=" "$log"; then
    echo "qemu-smoke: missing 'ident: heap' on serial (identity heap tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: heap-fault" "$log"; then
    echo "qemu-smoke: missing 'ident: heap-fault' on serial (EL1 identity heap DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: heap-high" "$log"; then
    echo "qemu-smoke: missing 'ident: heap-high' on serial (EL1 high heap load, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: start-stay" "$log"; then
    echo "qemu-smoke: missing 'ident: start-stay' on serial (_start / 0x4008_0000 stay, qemu exit $qemu_ec)" >&2
    exit 1
fi
if grep -q "ident: ram missed" "$log"; then
    echo "qemu-smoke: ident ram missed (leftover identity RAM stayed mapped)" >&2
    exit 1
fi
if grep -q "ident: miss ram-ready" "$log"; then
    echo "qemu-smoke: identity RAM tear did not publish" >&2
    exit 1
fi
if grep -q "ident: ram-stay" "$log"; then
    echo "qemu-smoke: ident ram-stay still printed (leftover identity RAM should be torn)" >&2
    exit 1
fi
if ! grep -q "ident: ram lo=" "$log"; then
    echo "qemu-smoke: missing 'ident: ram' on serial (leftover identity RAM tear, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: ram-fault" "$log"; then
    echo "qemu-smoke: missing 'ident: ram-fault' on serial (EL1 identity RAM DABORT, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "ident: ram-high" "$log"; then
    echo "qemu-smoke: missing 'ident: ram-high' on serial (EL1 high leftover RAM load, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: identity-tear strings present"
if grep -q "pan: probe missed" "$log"; then
    echo "qemu-smoke: pan probe missed (ID_AA64MMFR1_EL1.PAN was not published)" >&2
    exit 1
fi
if grep -q "pan: enabled" "$log"; then
    echo "qemu-smoke: pan enabled (PSTATE.PAN must stay off on -cpu cortex-a57)" >&2
    exit 1
fi
if ! grep -q "pan: id=" "$log"; then
    echo "qemu-smoke: missing 'pan: id=' on serial (PAN capability ID field, qemu exit $qemu_ec)" >&2
    exit 1
fi
if ! grep -q "pan: absent" "$log"; then
    echo "qemu-smoke: missing 'pan: absent' on serial (PAN unimplemented on -cpu cortex-a57, qemu exit $qemu_ec)" >&2
    exit 1
fi
echo "qemu-smoke: PAN capability strings present"
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

# A9 leftover: same published app ELF on this OS (already grepped) and
# documented prior OS ba6541c (A9 merge; earliest main tip with FAT /hello).
# Invent nothing: that SHA is real and has the slot path. File presence of
# a stored kernel blob is not this probe — we build that commit.
PRIOR_OS_SHA="${CTOS_PRIOR_OS_SHA:-ba6541c8e17c75c89c42451cd721735eddb36c2f}"
if ! command -v git >/dev/null 2>&1; then
    echo "qemu-smoke: git not on PATH (needed to build prior OS $PRIOR_OS_SHA)" >&2
    exit 1
fi
this_os=$(git rev-parse HEAD)
if [ "$this_os" = "$PRIOR_OS_SHA" ]; then
    echo "qemu-smoke: cross-update needs this OS != prior $PRIOR_OS_SHA" >&2
    exit 1
fi
if ! git cat-file -e "${PRIOR_OS_SHA}^{commit}" 2>/dev/null; then
    echo "qemu-smoke: prior OS $PRIOR_OS_SHA not in this clone; fetching from origin"
    # Shallow fetch of that SHA only. Do not invent a stored kernel blob.
    if ! git fetch --depth=1 origin "$PRIOR_OS_SHA"; then
        echo "qemu-smoke: missing prior OS commit $PRIOR_OS_SHA (do not invent a kernel blob)" >&2
        exit 1
    fi
fi
if ! git cat-file -e "${PRIOR_OS_SHA}^{commit}"; then
    echo "qemu-smoke: missing prior OS commit $PRIOR_OS_SHA (do not invent a kernel blob)" >&2
    exit 1
fi
cross_app=$(mktemp)
cross_img=$(mktemp)
cross_log=$(mktemp)
trap 'rm -f "$log" "$cross_app" "$cross_img" "$cross_log"' EXIT
cp -f "$app" "$cross_app"
app_hash=$(sha256_file "$cross_app" || true)
if [ -z "$app_hash" ] || [ "${#app_hash}" -lt 64 ]; then
    echo "qemu-smoke: could not hash app payload $cross_app (need sha256sum, shasum -a 256, or openssl)" >&2
    exit 1
fi
echo "qemu-smoke: cross-update app sha256=$app_hash bytes=$app_bytes"
# Outside this repo so cargo does not pick up *this* `.cargo/config.toml`
# (a nested worktree under target/ applied `-Tlinker.ld` twice and
# overlapped `.ident_tear` with `.text` on the prior OS).
prior_wt="${CTOS_PRIOR_OS_DIR:-/tmp/ctos-prior-os-$PRIOR_OS_SHA}"
if [ ! -f "$prior_wt/.git" ] && [ ! -d "$prior_wt/.git" ]; then
    # A Docker COPY of host `.git` can still carry worktree metadata
    # for this path without the /tmp directory. Prune the stale
    # registration, then add. Do not invent a kernel blob.
    git worktree prune
    echo "qemu-smoke: git worktree add prior OS $PRIOR_OS_SHA"
    git worktree add --detach "$prior_wt" "$PRIOR_OS_SHA"
fi
if [ "$(git -C "$prior_wt" rev-parse HEAD)" != "$PRIOR_OS_SHA" ]; then
    echo "qemu-smoke: prior worktree HEAD is not $PRIOR_OS_SHA" >&2
    exit 1
fi
echo "qemu-smoke: cargo build prior OS $PRIOR_OS_SHA"
(cd "$prior_wt" && cargo +nightly build)
prior_elf="$prior_wt/target/aarch64-ctos/debug/ctos"
if [ ! -x "$prior_elf" ]; then
    echo "qemu-smoke: missing prior kernel $prior_elf" >&2
    exit 1
fi
if cmp -s "$elf" "$prior_elf"; then
    echo "qemu-smoke: this OS and prior OS produced the same kernel ELF" >&2
    exit 1
fi
echo "qemu-smoke: QEMU virt serial + RX inject prior OS (timeout ${TIMEOUT_SECS}s)"
set +e
CTOS_QEMU_TIMEOUT="$TIMEOUT_SECS" CTOS_BLK_IMAGE="$cross_img" CTOS_APP_ELF="$cross_app" \
    python3 "$ROOT/scripts/qemu-serial-inject.py" "$prior_elf" >"$cross_log" 2>&1
prior_ec=$?
set -e
cat "$cross_log"
if grep -q "slot: probe missed" "$cross_log"; then
    echo "qemu-smoke: prior OS slot probe missed (same app on $PRIOR_OS_SHA)" >&2
    exit 1
fi
if grep -q "slot: embed" "$cross_log"; then
    echo "qemu-smoke: prior OS slot used the A3 embed (must be FAT /hello)" >&2
    exit 1
fi
if ! grep -q "slot: fat" "$cross_log"; then
    echo "qemu-smoke: missing 'slot: fat' on prior OS $PRIOR_OS_SHA (qemu exit $prior_ec)" >&2
    exit 1
fi
if ! grep -q "slot: mapped" "$cross_log"; then
    echo "qemu-smoke: missing 'slot: mapped' on prior OS $PRIOR_OS_SHA (qemu exit $prior_ec)" >&2
    exit 1
fi
if ! grep -q "slot: ok" "$cross_log"; then
    echo "qemu-smoke: missing 'slot: ok' on prior OS $PRIOR_OS_SHA (qemu exit $prior_ec)" >&2
    exit 1
fi
echo "qemu-smoke: cross-update prior-os=$PRIOR_OS_SHA slot:ok"
echo "qemu-smoke: cross-update this-os=$this_os slot:ok"
echo "qemu-smoke: A9 cross-update (same app sha256=$app_hash) strings present"
# cargo test rebuilds target/fat16.img from the published apps.
export CTOS_APP_ELF="$app"
export CTOS_APP2_ELF="$app2"
export CTOS_APP3_ELF="$app3"
export CTOS_APP4_ELF="$app4"
export CTOS_BLK_IMAGE="$img"

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

# Force-fail rebuilds ctos with --features force-fail. On ubuntu-24.04-arm GHA
# that recompile alone can take ~30s, and ADR-059 /fsdemo + ADR-061 /fatdemo + ADR-062 /yldemo also lengthen boot —
# so the old 30s wall often expired before semihosting exit. Override via env.
FORCE_FAIL_TIMEOUT_SECS="${CTOS_FORCE_FAIL_TIMEOUT:-120}"
echo "qemu-smoke: force-fail must be non-zero (timeout ${FORCE_FAIL_TIMEOUT_SECS}s)"
set +e
timeout "$FORCE_FAIL_TIMEOUT_SECS" cargo +nightly test --features force-fail -- --nocapture
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
