#!/usr/bin/env python3
"""QEMU guestfwd stdio peer for ADR-074 thin TCP (N5).

stdin/stdout are the TCP byte stream. Read exactly one probe payload
(b"ctos-tcp\\n"), echo it, exit (peer FIN). No external network.
"""
from __future__ import annotations

import sys

NEED = 9  # len(b"ctos-tcp\n")
EXPECTED = b"ctos-tcp\n"


def main() -> int:
    buf = bytearray()
    while len(buf) < NEED:
        chunk = sys.stdin.buffer.read(NEED - len(buf))
        if not chunk:
            break
        buf.extend(chunk)
    if bytes(buf) != EXPECTED:
        # Still echo what we got so a wrong guest can be diagnosed in captures;
        # non-zero exit marks host peer unhappy.
        if buf:
            sys.stdout.buffer.write(buf)
            sys.stdout.buffer.flush()
        return 1
    sys.stdout.buffer.write(buf)
    sys.stdout.buffer.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
