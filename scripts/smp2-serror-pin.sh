#!/usr/bin/env bash
# ADR-110 (N1): taken SError on core 1 at EL0 with the pinned QEMU (ADR-083).
# Needs CTOS_QEMU = pinned qemu-system-aarch64 (TYPE_NMI patch). Fail-closed.
#  P  smp2-serror-probe,  nmi cpu-index=1 -> cpu1 takes it (taken=yes)
#  N0 smp2-serror-probe,  nmi cpu-index=0 -> cpu1 must NOT take it (park)
#  NM smp2-serror-masked, nmi cpu-index=1 -> PSTATE.A set: NOT taken (park)
# Every row also requires the host nmi to have returned ok, S4 to have
# passed and no unhandled SError on either core.
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
: "${CTOS_QEMU:?set CTOS_QEMU to the pinned qemu-system-aarch64 (ADR-083)}"
export CTOS_QEMU
T="${CTOS_SMP2_DEV_TIMEOUT:-180}"
cargo build --features smp2-serror-probe --target-dir target/smp2-serror
cargo build --features smp2-serror-masked --target-dir target/smp2-serror-masked
POS=target/smp2-serror/aarch64-ctos/debug/ctos
MSK=target/smp2-serror-masked/aarch64-ctos/debug/ctos
fail=0
row() { # name elf cpu expect forbid [expect2]
    local name=$1 elf=$2 cpu=$3 expect=$4 forbid=$5 expect2=${6:-} log
    log=$(mktemp)
    CTOS_QEMU_SMP=2 CTOS_QEMU_STOP_AT="smp: done" CTOS_QEMU_TIMEOUT="$T" \
    CTOS_QEMU_NMI_CPU1="$cpu" \
        python3 scripts/qemu-serial-inject.py "$elf" >"$log" 2>&1 || true
    tr -d '\r' <"$log" >"$log.c"
    local ok=1
    grep -aqF "nmi(cpu-index=$cpu)=>ok" "$log.c" || { echo "smp2-serror: $name: host nmi not ok"; ok=0; }
    grep -aqE "$expect" "$log.c" || { echo "smp2-serror: $name: missing /$expect/"; ok=0; }
    if [ -n "$expect2" ] && ! grep -aqE "$expect2" "$log.c"; then echo "smp2-serror: $name: missing /$expect2/"; ok=0; fi
    if grep -aqE "$forbid" "$log.c"; then echo "smp2-serror: $name: forbidden /$forbid/"; ok=0; fi
    grep -aq '^smp: ok cpu1 s4 ' "$log.c" || { echo "smp2-serror: $name: S4 not ok"; ok=0; }
    if grep -aq 'unhandled lower serror\|unhandled vector' "$log.c"; then echo "smp2-serror: $name: unhandled exception"; ok=0; fi
    grep -a 'cpu1 serror\|cpu1 el0 serror\|cpu1-arm' "$log.c" | sed "s/^/smp2-serror: $name | /"
    if [ $ok = 1 ]; then echo "smp2-serror: ok   $name"; else echo "smp2-serror: FAIL $name"; fail=1; tail -n 40 "$log.c"; fi
    rm -f "$log" "$log.c"
}
TAKEN='^smp: cpu1 el0 serror taken cpu=1[[:space:]]*$'
row "P  nmi->cpu1, cpu1 at EL0" "$POS" 1 '^smp: cpu1 serror taken=yes masked=no cpu=1' '^smp: cpu1 serror park' "$TAKEN"
row "N0 nmi->cpu0, cpu1 at EL0" "$POS" 0 '^smp: cpu1 serror park taken=no masked=no cpu=1' "$TAKEN"
row "NM cpu1 A masked"         "$MSK" 1 '^smp: cpu1 serror park taken=no masked=yes cpu=1' "$TAKEN"
if [ $fail = 0 ]; then echo "smp2-serror: ok (cpu1 taken on nmi cpu-index=1; not taken when targeted at cpu0 or masked)"; else echo "smp2-serror: FAILED" >&2; exit 1; fi
