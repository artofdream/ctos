#!/usr/bin/env python3
"""Run the hello kernel and inject one PL011 RX byte after Hello World!.

QEMU 8.2's PL011 does not implement UARTCR.LBE. The M6 probe is a real
chardev byte on `-serial stdio`. Exit 124 if we kill the wfe loop
(same as timeout(1)).
"""
from __future__ import annotations

import os
import select
import subprocess
import sys
import time

HELLO = os.environ.get("CTOS_HELLO_STRING", "Hello World!").encode()
INJECT = bytes([int(os.environ.get("CTOS_INPUT_BYTE", "0x41"), 0)])
TIMEOUT = float(os.environ.get("CTOS_QEMU_TIMEOUT", "8"))


def main() -> int:
    if len(sys.argv) < 2:
        print("qemu-serial-inject: missing kernel ELF", file=sys.stderr)
        return 1
    elf = sys.argv[1]
    cmd = [
        "qemu-system-aarch64",
        "-machine",
        "virt",
        "-cpu",
        "cortex-a57",
        "-m",
        "128M",
        "-display",
        "none",
        "-serial",
        "stdio",
        "-semihosting",
        "-kernel",
        elf,
    ]
    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    assert proc.stdin is not None
    assert proc.stdout is not None
    fd = proc.stdout.fileno()
    os.set_blocking(fd, False)

    deadline = time.monotonic() + TIMEOUT
    buf = bytearray()
    injected = False
    while time.monotonic() < deadline:
        timeout_left = deadline - time.monotonic()
        if timeout_left <= 0:
            break
        ready, _, _ = select.select([fd], [], [], min(0.05, timeout_left))
        if ready:
            chunk = os.read(fd, 4096)
            if chunk:
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
                buf += chunk
                if not injected and HELLO in buf:
                    proc.stdin.write(INJECT)
                    proc.stdin.flush()
                    injected = True
            elif proc.poll() is not None:
                break
        if proc.poll() is not None:
            rest = os.read(fd, 4096)
            if rest:
                sys.stdout.buffer.write(rest)
                sys.stdout.buffer.flush()
            return proc.returncode if proc.returncode is not None else 1

    if proc.poll() is None:
        proc.kill()
        try:
            proc.wait(timeout=1)
        except subprocess.TimeoutExpired:
            proc.terminate()
        rest = b""
        try:
            rest = os.read(fd, 4096)
        except OSError:
            rest = b""
        if rest:
            sys.stdout.buffer.write(rest)
            sys.stdout.buffer.flush()
        return 124
    return proc.returncode if proc.returncode is not None else 1


if __name__ == "__main__":
    sys.exit(main())
