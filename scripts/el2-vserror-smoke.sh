#!/usr/bin/env bash
# ADR-111 (N2): taken *virtual* SError at EL0 with the full ctos image on
# stock QEMU, via the EL2 shim (tools/el2-shim). Fail-closed. Opt-in: not
# part of qemu-smoke.sh or any required check.
#  P   el2-vserror build, shim, virt,virtualization=on -> virtual serror taken
#  NA  default build (no ask), shim                    -> park, no virtual line
#  NM  el2-vserror-masked build, shim                  -> ask vse=set, park
#  NS  el2-vserror build, no shim, plain virt (no EL2) -> el2-shim=no, park
# Every row must reach `el0: ok` with no unhandled exception.
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
T="${CTOS_EL2_TIMEOUT:-240}"
SHIM="$ROOT/tools/el2-shim/target/aarch64-unknown-none/release/el2-shim"
if [ "${CTOS_EL2_NO_BUILD:-0}" != 1 ]; then  # reuse earlier builds (other QEMU)
    (cd tools/el2-shim && cargo build --release)
    cargo build
    cargo build --features el2-vserror --target-dir target/el2-vserror
    cargo build --features el2-vserror-masked --target-dir target/el2-vserror-masked
fi
echo "el2-vserror: qemu=$("${CTOS_QEMU:-qemu-system-aarch64}" --version | head -n 1)"
DEF=target/aarch64-ctos/debug/ctos
POS=target/el2-vserror/aarch64-ctos/debug/ctos
MSK=target/el2-vserror-masked/aarch64-ctos/debug/ctos
SHIMARGS="-device loader,file=$SHIM,cpu-num=0"
fail=0
row() { # name elf machine extra expect forbid [expect2]
    local name=$1 elf=$2 mach=$3 extra=$4 expect=$5 forbid=$6 expect2=${7:-} log ok=1
    log=$(mktemp)
    CTOS_QEMU_MACHINE="$mach" CTOS_QEMU_EXTRA="$extra" CTOS_QEMU_STOP_AT="el0: ok" \
    CTOS_QEMU_TIMEOUT="$T" python3 scripts/qemu-serial-inject.py "$elf" >"$log" 2>&1 || true
    tr -d '\r' <"$log" >"$log.c"
    grep -aqE "$expect" "$log.c" || { echo "el2-vserror: $name: missing /$expect/"; ok=0; }
    if [ -n "$expect2" ] && ! grep -aqE "$expect2" "$log.c"; then echo "el2-vserror: $name: missing /$expect2/"; ok=0; fi
    if grep -aqE "$forbid" "$log.c"; then echo "el2-vserror: $name: forbidden /$forbid/"; ok=0; fi
    grep -aq '^el0: ok' "$log.c" || { echo "el2-vserror: $name: no el0: ok"; ok=0; }
    if grep -aq 'unhandled\|el2-shim: unexpected' "$log.c"; then echo "el2-vserror: $name: unhandled exception"; ok=0; fi
    grep -a 'el2-shim\|virtual serror\|serror-park\|^el0: serror' "$log.c" | sed "s/^/el2-vserror: $name | /"
    if [ $ok = 1 ]; then echo "el2-vserror: ok   $name"; else echo "el2-vserror: FAIL $name"; fail=1; tail -n 30 "$log.c"; fi
    rm -f "$log" "$log.c"
}
TAKEN='^el0: virtual serror taken \(el2 vse\) ec=0x2f'
V=virt,virtualization=on
row "P  shim + ask"         "$POS" "$V" "$SHIMARGS" "$TAKEN" '^el0: serror-park' '^el0: virtual serror ask el2-shim=yes vse=set masked=no'
row "NA shim, default build" "$DEF" "$V" "$SHIMARGS" '^el0: serror-park' 'virtual serror' '^el2-shim: el1'
row "NM shim + ask, A masked" "$MSK" "$V" "$SHIMARGS" '^el0: virtual serror ask el2-shim=yes vse=set masked=yes' "$TAKEN" '^el0: serror-park'
row "NS no shim, no EL2"    "$POS" virt "" '^el0: virtual serror ask el2-shim=no vse=no masked=no' "$TAKEN|el2-shim: el1" '^el0: serror-park'
if [ $fail = 0 ]; then echo "el2-vserror: ok (virtual SError taken at EL0 only with the shim + ask + A clear)"; else echo "el2-vserror: FAILED" >&2; exit 1; fi
