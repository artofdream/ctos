#!/bin/sh
# ADR-074 / N5 — QEMU guestfwd cmd peer (path must stay comma-free for -netdev).
# stdin/stdout = TCP stream from guest active-open to 10.0.2.4:7.
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 -u "$ROOT/tcp-echo-stdio.py"
