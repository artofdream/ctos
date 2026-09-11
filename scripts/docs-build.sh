#!/usr/bin/env bash
# Build the mdBook docs site locally (FR-14 / NFR-13).
# A green local build is not "the website is published."
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MDBOOK_VERSION="${MDBOOK_VERSION:-0.5.4}"
MDBOOK_MERMAID_VERSION="${MDBOOK_MERMAID_VERSION:-0.17.1}"

tmp=""
ensure_tmp() {
  if [ -z "$tmp" ]; then
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
  fi
}

linux_triple() {
  local tool="$1"
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) echo "x86_64-unknown-linux-gnu" ;;
    aarch64|arm64)
      # Official mdBook + mdbook-mermaid Linux ARM assets are musl.
      echo "aarch64-unknown-linux-musl"
      ;;
    *)
      echo "docs-build: install $tool for $arch, then re-run" >&2
      exit 1
      ;;
  esac
}

need_mdbook=0
if ! command -v mdbook >/dev/null 2>&1; then
  need_mdbook=1
else
  have="$(mdbook --version | awk '{print $2}')"
  if [ "$have" != "$MDBOOK_VERSION" ]; then
    echo "docs-build: found mdbook $have (want $MDBOOK_VERSION); using PATH binary anyway" >&2
  fi
fi

if [ "$need_mdbook" -eq 1 ]; then
  ensure_tmp
  triple="$(linux_triple "mdBook $MDBOOK_VERSION")"
  archive="mdbook-v${MDBOOK_VERSION}-${triple}.tar.gz"
  echo "docs-build: downloading mdBook ${MDBOOK_VERSION} (${triple})"
  curl -sSL "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/${archive}" \
    | tar -xz -C "$tmp"
  export PATH="$tmp:$PATH"
fi

need_mermaid=0
if ! command -v mdbook-mermaid >/dev/null 2>&1; then
  need_mermaid=1
fi

if [ "$need_mermaid" -eq 1 ]; then
  ensure_tmp
  triple="$(linux_triple "mdbook-mermaid $MDBOOK_MERMAID_VERSION")"
  archive="mdbook-mermaid-v${MDBOOK_MERMAID_VERSION}-${triple}.tar.gz"
  echo "docs-build: downloading mdbook-mermaid ${MDBOOK_MERMAID_VERSION} (${triple})"
  curl -sSL "https://github.com/badboy/mdbook-mermaid/releases/download/v${MDBOOK_MERMAID_VERSION}/${archive}" \
    | tar -xz -C "$tmp"
  export PATH="$tmp:$PATH"
fi

echo "docs-build: $(mdbook --version); mdbook-mermaid $(mdbook-mermaid --version 2>/dev/null || echo present)"
mdbook build
test -f book/CNAME
test "$(tr -d '[:space:]' < book/CNAME)" = "ctos.artof.link"
# mdBook 0.5 hashes extra CSS/JS; presence of mermaid assets in the book tree is enough.
test -n "$(find book \( -name '*mermaid-init*' -o -name '*mermaid.css' \) | head -n 1)"
# Website chapters are these docs/ files — fail if the book dropped one.
for html in \
  book/overview/what-can-run.html \
  book/overview/porting.html \
  book/overview/filesystem.html \
  book/overview/hosting-apps.html \
  book/overview/measure.html \
  book/overview/prerequisites.html \
  book/overview/advantages.html \
  book/overview/limits.html
do
  test -f "$html"
done
# Preprocessor must have left at least one mermaid node (not only a fence in source).
if ! grep -q 'class="mermaid"' book/index.html book/overview/*.html; then
  echo "docs-build: expected a rendered mermaid node in landing or overview HTML" >&2
  exit 1
fi
echo "docs-build: ok (output ./book/ — generator only; live URL is a separate ledger row)"
