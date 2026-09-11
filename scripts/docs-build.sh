#!/usr/bin/env bash
# Build the mdBook docs site locally (FR-14 / NFR-13).
# A green local build is not "the website is published."
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MDBOOK_VERSION="${MDBOOK_VERSION:-0.5.4}"

need_install=0
if ! command -v mdbook >/dev/null 2>&1; then
  need_install=1
else
  have="$(mdbook --version | awk '{print $2}')"
  if [ "$have" != "$MDBOOK_VERSION" ]; then
    echo "docs-build: found mdbook $have (want $MDBOOK_VERSION); using PATH binary anyway" >&2
  fi
fi

if [ "$need_install" -eq 1 ]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) triple="x86_64-unknown-linux-gnu" ;;
    aarch64|arm64) triple="aarch64-unknown-linux-musl" ;;
    *)
      echo "docs-build: install mdBook $MDBOOK_VERSION for $arch, then re-run" >&2
      echo "  https://github.com/rust-lang/mdBook/releases/tag/v${MDBOOK_VERSION}" >&2
      exit 1
      ;;
  esac
  archive="mdbook-v${MDBOOK_VERSION}-${triple}.tar.gz"
  echo "docs-build: downloading mdBook ${MDBOOK_VERSION} (${triple})"
  curl -sSL "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/${archive}" \
    | tar -xz -C "$tmp"
  export PATH="$tmp:$PATH"
fi

echo "docs-build: $(mdbook --version)"
mdbook build
test -f book/CNAME
test "$(tr -d '[:space:]' < book/CNAME)" = "ctos.artof.link"
# Website chapters are these docs/ files — fail if the book dropped one.
for html in \
  book/overview/what-can-run.html \
  book/overview/porting.html \
  book/overview/filesystem.html \
  book/overview/measure.html \
  book/overview/prerequisites.html \
  book/overview/advantages.html \
  book/overview/limits.html
do
  test -f "$html"
done
echo "docs-build: ok (output ./book/ — not a live Pages probe)"
