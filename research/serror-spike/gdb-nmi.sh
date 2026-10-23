#!/usr/bin/env bash
# ADR-109: gdbstub route. gdb `monitor nmi` is forwarded (qRcmd) to the same
# HMP `nmi` as QMP inject-nmi; gdb itself has no way to raise an SError.
# Usage: CTOS_QEMU=<qemu> gdb-nmi.sh <vse-spike elf>   (mode 4: EL0 spin)
set -u
Q="${CTOS_QEMU:-qemu-system-aarch64}"; ELF="$1"; LOG=$(mktemp)
"$Q" -M virt,virtualization=on -cpu cortex-a76 -nographic -monitor none \
  -serial file:"$LOG" -gdb tcp::12345 -kernel "$ELF" \
  -device loader,addr=0x40200000,data=4,data-len=1 & QP=$!
sleep 2
gdb-multiarch -q -batch -ex 'target remote :12345' -ex 'monitor nmi' -ex 'continue &' \
  -ex 'shell sleep 1' -ex 'detach' 2>&1 | grep -iv '^warning\|0x' | sed 's/^/gdb: /'
sleep 1; kill $QP 2>/dev/null; wait $QP 2>/dev/null
tr -d '\r' < "$LOG" | sed 's/^/serial: /'; rm -f "$LOG"
