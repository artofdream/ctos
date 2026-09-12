#!/bin/sh
# Build and run the smoke image on whatever arch the engine is.
# cts-ai: Windows ARM64 → linux/arm64. Do not pass --platform linux/amd64.
# Leftover cross-update needs git + .git in the image (ADR-032).
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

if ! command -v docker >/dev/null 2>&1; then
    echo "docker-smoke: docker not on PATH" >&2
    exit 1
fi

docker build -t ctos-smoke .
docker run --rm ctos-smoke
