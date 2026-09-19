#!/usr/bin/env python3
"""ADR-082 B1: probe whether QEMU virt exposes TYPE_NMI via inject-nmi.

Fail-soft research helper. Does not boot ctos. Does not claim SError Verified.
Exit 0 always after printing the matrix (research tool, not a CI gate).
"""
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
from typing import Any


ROWS: list[tuple[str, str, str]] = [
    ("virt", "cortex-a76", "default-smoke"),
    ("virt", "cortex-a57", "historical-a57"),
    ("virt,gic-version=3", "cortex-a76", "gicv3-a76"),
    ("virt,gic-version=3", "max", "gicv3-max"),
    ("virt,virtualization=on", "cortex-a76", "virt-on"),
    ("virt,gic-version=3,virtualization=on", "cortex-a76", "gicv3-virt-on"),
    ("virt,gic-version=3,secure=on", "cortex-a76", "gicv3-secure"),
]


def qmp_inject(machine: str, cpu: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="ctos-nmi-") as d:
        sock_path = os.path.join(d, "qmp.sock")
        qemu = os.environ.get("CTOS_QEMU", "qemu-system-aarch64")
        cmd = [
            qemu,
            "-machine",
            machine,
            "-cpu",
            cpu,
            "-m",
            "64M",
            "-display",
            "none",
            "-S",
            "-qmp",
            f"unix:{sock_path},server,nowait",
        ]
        proc = subprocess.Popen(
            cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE
        )
        try:
            deadline = time.monotonic() + 5.0
            while time.monotonic() < deadline:
                if os.path.exists(sock_path):
                    break
                if proc.poll() is not None:
                    err = (proc.stderr.read() or b"").decode(errors="replace")[:400]
                    return {"start_error": err.strip() or f"exit={proc.returncode}"}
                time.sleep(0.05)
            else:
                return {"start_error": "qmp socket timeout"}

            sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            sock.settimeout(3.0)
            sock.connect(sock_path)
            buf = b""

            def recv() -> dict[str, Any]:
                nonlocal buf
                while b"\n" not in buf:
                    chunk = sock.recv(4096)
                    if not chunk:
                        raise ConnectionError("qmp eof")
                    buf += chunk
                line, buf = buf.split(b"\n", 1)
                return json.loads(line.decode())

            greet = recv()
            sock.sendall(b'{"execute":"qmp_capabilities"}\n')
            caps = recv()
            sock.sendall(b'{"execute":"inject-nmi"}\n')
            inj = recv()
            sock.sendall(b'{"execute":"quit"}\n')
            try:
                sock.close()
            except OSError:
                pass
            return {
                "qemu": greet.get("QMP", {}).get("version", {}).get("qemu"),
                "caps": caps,
                "inject-nmi": inj,
            }
        finally:
            if proc.poll() is None:
                proc.kill()
                try:
                    proc.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    proc.terminate()


def main() -> int:
    print(f"qemu-nmi-probe: host={socket.gethostname()}")
    try:
        ver = subprocess.check_output(
            [os.environ.get("CTOS_QEMU", "qemu-system-aarch64"), "--version"], text=True
        ).splitlines()[0]
    except (FileNotFoundError, subprocess.CalledProcessError) as e:
        print(f"qemu-nmi-probe: missing qemu-system-aarch64: {e}", file=sys.stderr)
        return 1
    print(f"qemu-nmi-probe: {ver}")
    print("qemu-nmi-probe: ADR-082 B1 research — TYPE_NMI availability (not SError Verified)")
    print()
    print("| label | machine | cpu | inject-nmi |")
    print("| --- | --- | --- | --- |")
    for machine, cpu, label in ROWS:
        result = qmp_inject(machine, cpu)
        if "start_error" in result:
            cell = f"START_FAIL: {result['start_error']}"
        else:
            inj = result.get("inject-nmi") or {}
            if "return" in inj:
                cell = "ok (TYPE_NMI present — still must prove guest el0: serror)"
            else:
                err = (inj.get("error") or {}).get("desc") or str(inj)
                cell = f"error: {err}"
        print(f"| {label} | `{machine}` | `{cpu}` | {cell} |")
    print()
    print(
        "qemu-nmi-probe: done. FEAT_NMI / GICv3 NMI ≠ async SError. "
        "Do not claim taken Verified from this script alone."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
