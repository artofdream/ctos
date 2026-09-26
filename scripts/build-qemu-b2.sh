#!/usr/bin/env bash
# Build QEMU v10.0.0 + B1 TCG patch (0001) + B2 KVM serror patch (0002) into
# tools/qemu-b2/ (ADR-085 / B2-P). On arm64 hosts configure auto-detects KVM.
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX="${CTOS_QEMU_PREFIX:-$ROOT/tools/qemu-b2}"
TAG="${CTOS_QEMU_TAG:-v10.0.0}"
COMMIT="${CTOS_QEMU_COMMIT:-7c949c53e936aa3a658d84ab53bae5cadaa5d59c}"
P1="$ROOT/research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch"
P2="$ROOT/research/qemu-nmi/0002-hw-arm-virt-TYPE_NMI-KVM-serror-pending.patch"
SRC="${CTOS_QEMU_SRC:-$ROOT/target/qemu-b2-src}"
JOBS="${CTOS_QEMU_JOBS:-$(nproc 2>/dev/null || echo 2)}"

echo "build-qemu-b2: tag=$TAG commit=$COMMIT prefix=$PREFIX jobs=$JOBS"
mkdir -p "$PREFIX" "$(dirname "$SRC")"
if [ ! -d "$SRC/.git" ]; then
  git clone --depth 1 --branch "$TAG" https://gitlab.com/qemu-project/qemu.git "$SRC"
fi
cd "$SRC"
git fetch --depth 1 origin "$TAG" 2>/dev/null || true
git checkout -f "$COMMIT" 2>/dev/null || git checkout -f "$TAG"
git submodule update --init --recursive --depth 1 || true
git reset --hard HEAD
git clean -fdx -e build 2>/dev/null || true
for P in "$P1" "$P2"; do
  if git apply --check "$P" 2>/dev/null; then
    git apply "$P"; echo "build-qemu-b2: applied $(basename "$P")"
  elif git apply --reverse --check "$P" 2>/dev/null; then
    echo "build-qemu-b2: $(basename "$P") already applied"
  else
    echo "build-qemu-b2: patch failed: $(basename "$P")" >&2; exit 1
  fi
done
mkdir -p build && cd build
../configure \
  --target-list=aarch64-softmmu \
  --disable-docs --disable-user --disable-tools --disable-guest-agent \
  --disable-sdl --disable-gtk --disable-vnc \
  --enable-slirp \
  --prefix="$PREFIX"
ninja -j"$JOBS" qemu-system-aarch64
ninja install
test -x "$PREFIX/bin/qemu-system-aarch64"
"$PREFIX/bin/qemu-system-aarch64" --version | head -1
echo "build-qemu-b2: accelerators: $("$PREFIX/bin/qemu-system-aarch64" -accel help | tr '\n' ' ')"
echo "build-qemu-b2: ok -> $PREFIX/bin/qemu-system-aarch64"
