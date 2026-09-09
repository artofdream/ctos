# Session memory — 2026-09-09 (Docker CRLF + missing cc)

cts-ai: `docker build -t ctos-smoke .` on Docker Desktop linux/arm64 succeeded. First `docker run` failed: `exec ./scripts/qemu-smoke.sh: no such file or directory` (`#!/bin/sh\r`). After LF, second `docker run` failed: `linker \`cc\` not found` compiling `compiler_builtins` (nightly build-std). Ratchets: `.gitattributes` eol=lf; image `sed` + `CMD bash`; `build-essential`. Full smoke after both still unprobed here.
