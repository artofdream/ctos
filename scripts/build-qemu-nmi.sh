#!/usr/bin/env bash
# Build pinned QEMU v10.0.0 + TYPE_NMI→SError patch into tools/qemu-nmi/ (ADR-083).
set -euo pipefail
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX="${CTOS_QEMU_PREFIX:-$ROOT/tools/qemu-nmi}"
TAG="${CTOS_QEMU_TAG:-v10.0.0}"
COMMIT="${CTOS_QEMU_COMMIT:-7c949c53e936aa3a658d84ab53bae5cadaa5d59c}"
PATCH="$ROOT/research/qemu-nmi/0001-hw-arm-virt-TYPE_NMI-raise-SError.patch"
SRC="${CTOS_QEMU_SRC:-$ROOT/target/qemu-nmi-src}"
JOBS="${CTOS_QEMU_JOBS:-$(nproc 2>/dev/null || echo 2)}"

echo "build-qemu-nmi: tag=$TAG commit=$COMMIT prefix=$PREFIX"
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
# Apply patch (allow already-applied)
if git apply --check "$PATCH" 2>/dev/null; then
  git apply "$PATCH"
elif git apply --reverse --check "$PATCH" 2>/dev/null; then
  echo "build-qemu-nmi: patch already applied"
else
  git apply "$PATCH" || { echo "build-qemu-nmi: patch failed" >&2; exit 1; }
fi
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
echo "build-qemu-nmi: ok → $PREFIX/bin/qemu-system-aarch64"
echo "build-qemu-nmi: export CTOS_QEMU=$PREFIX/bin/qemu-system-aarch64 CTOS_REQUIRE_TAKEN_SERROR=1"
