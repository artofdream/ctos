#!/bin/bash
# ADR-085 B2-P no-SSH run on a Graviton .metal (KVM). Prints markers to the
# serial console (read via `aws ec2 get-console-output --latest`), then
# powers off. instance-initiated-shutdown-behavior=terminate removes it.
# Hard self-terminate timer: guaranteed poweroff even if a step hangs.
shutdown -h +80 "ctos b2 hard timer" || true
exec > >(tee /dev/console /var/log/ctos-b2.log) 2>&1
set -x
# cloud-init runs user-data with HOME unset (run 1, 2026-09-26: `. /.cargo/env`
# not found, rustc missing). Pin HOME and the cargo bin dir explicitly.
export HOME=/root
export PATH=/root/.cargo/bin:$PATH
echo "CTOSB2: start $(date -u +%FT%TZ)"
echo "CTOSB2: uname $(uname -a)"
echo "CTOSB2: cpu $(lscpu | grep -E 'Model name|Vendor' | tr -s ' ' | tr '\n' ';')"
dmesg | grep -iE 'kvm|hyp|EL2' | sed 's/^/CTOSB2: dmesg /' | head -20
if [ -e /dev/kvm ]; then echo "CTOSB2: /dev/kvm=present"; ls -l /dev/kvm; else echo "CTOSB2: /dev/kvm=absent"; fi
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq git curl build-essential ninja-build meson pkg-config python3-venv \
  flex bison libglib2.0-dev libpixman-1-dev libfdt-dev libslirp-dev >/dev/null
echo "CTOSB2: deps rc=$?"
# Rust nightly for the b2-serror guest (native aarch64 build).
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain nightly \
  -c rust-src,llvm-tools-preview >/dev/null 2>&1
. /root/.cargo/env || true
echo "CTOSB2: rustup $(rustup --version 2>&1 | head -1)"
cd /root
git clone --depth 1 --branch "__BRANCH__" https://github.com/artofdream/ctos.git ctos
cd ctos
echo "CTOSB2: head $(git rev-parse HEAD)"
echo "CTOSB2: rustc $(rustc --version 2>&1)"
CTOS_QEMU_JOBS=$(nproc) ./scripts/build-qemu-b2.sh > /var/log/ctos-qemu.log 2>&1
echo "CTOSB2: qemu-build rc=$?"
tail -3 /var/log/ctos-qemu.log | sed 's/^/CTOSB2: qemu /'
cargo build --features b2-serror --target-dir target-b2 > /var/log/ctos-cargo.log 2>&1
echo "CTOSB2: cargo rc=$?"
tail -2 /var/log/ctos-cargo.log | sed 's/^/CTOSB2: cargo /'
# Causal control (diagnostic only, never a pass): same profile but keeps the
# default pre-MMU spin locks (feature b2-prelock-probe).
cargo build --features b2-prelock-probe --target-dir target-b2p > /var/log/ctos-cargo-p.log 2>&1
echo "CTOSB2: cargo-prelock rc=$?"
QEMU_B2=$PWD/tools/qemu-b2/bin/qemu-system-aarch64
echo "CTOSB2: ===== control: b2-prelock-probe (expect silence if pre-MMU exclusives hang) ====="
CTOS_QEMU=$QEMU_B2 CTOS_B2_TIMEOUT=40 CTOS_B2_STALL=15 \
  python3 scripts/b2-kvm-smoke.py target-b2p/aarch64-ctos/debug/ctos 2>&1 | tr -d '\r' | sed 's/^/CTOSB2: ctl: /'
echo "CTOSB2: control rc=${PIPESTATUS[0]} (diagnostic only)"
ELF=target-b2/aarch64-ctos/debug/ctos
for i in 1 2 3; do
  echo "CTOSB2: ===== smoke attempt $i ====="
  CTOS_QEMU=$QEMU_B2 CTOS_B2_TIMEOUT=90 CTOS_B2_STALL=15 \
    python3 scripts/b2-kvm-smoke.py "$ELF" 2>&1 | tr -d '\r' | sed 's/^/CTOSB2: /'
  rc=${PIPESTATUS[0]}
  echo "CTOSB2: smoke attempt $i rc=$rc"
  [ "$rc" = 0 ] && break
done
echo "CTOSB2: end $(date -u +%FT%TZ)"
sync
sleep 20
poweroff
