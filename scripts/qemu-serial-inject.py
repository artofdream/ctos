#!/usr/bin/env python3
"""Run the hello kernel and inject one PL011 RX byte after Hello World!.

QEMU 8.2's PL011 does not implement UARTCR.LBE. The M6 probe is a real
chardev byte on `-serial stdio`. Exit 124 if we kill the wfe loop
(same as timeout(1)).

ADR-045: also open a QMP unix socket and, after the guest prints
`el0: serror-arm`, attempt `inject-nmi` (research H1 from ADR-044).
On QEMU 10 virt + `-cpu cortex-a57` this typically errors with
"machine does not provide NMIs" — log that honestly; do not fake a
taken SError. Guest keeps `el0: serror-park` when inject does not land.
"""
from __future__ import annotations

import json
import os
import select
import socket
import subprocess
import sys
import tempfile
import time

HELLO = os.environ.get("CTOS_HELLO_STRING", "Hello World!").encode()
SERROR_ARM = b"el0: serror-arm"
INJECT = bytes([int(os.environ.get("CTOS_INPUT_BYTE", "0x41"), 0)])
TIMEOUT = float(os.environ.get("CTOS_QEMU_TIMEOUT", "8"))
# Give the guest a moment after the cue to ERET into the A-clear spin.
SERROR_INJECT_DELAY = float(os.environ.get("CTOS_SERROR_INJECT_DELAY", "0.05"))


def prepare_fat16(root: str) -> str:
    """Host-visible FAT16 image for A7 + A9 /hello. Not a guest probe by itself."""
    img = os.environ.get("CTOS_BLK_IMAGE") or os.path.join(root, "target", "fat16.img")
    app = os.environ.get("CTOS_APP_ELF") or os.path.join(root, "target", "hello-libctos.elf")
    mk = os.path.join(root, "scripts", "mkfat16.py")
    if not os.path.isfile(app):
        raise SystemExit(f"qemu-serial-inject: missing app ELF {app} (A9 / ADR-030)")
    subprocess.check_call(
        [sys.executable, mk, "--app", app, img], stdout=subprocess.DEVNULL
    )
    return img


class QmpClient:
    """Minimal QMP client for inject-nmi (ADR-045). Fail-soft on errors."""

    def __init__(self, path: str) -> None:
        self.path = path
        self.sock: socket.socket | None = None
        self.buf = b""

    def connect(self, deadline: float) -> bool:
        while time.monotonic() < deadline:
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.connect(self.path)
                s.setblocking(False)
                self.sock = s
                return True
            except OSError:
                time.sleep(0.02)
        return False

    def _read_json(self, deadline: float) -> dict | None:
        assert self.sock is not None
        while time.monotonic() < deadline:
            if b"\n" in self.buf:
                line, self.buf = self.buf.split(b"\n", 1)
                line = line.strip()
                if not line:
                    continue
                return json.loads(line.decode())
            ready, _, _ = select.select([self.sock], [], [], 0.05)
            if not ready:
                continue
            try:
                chunk = self.sock.recv(4096)
            except BlockingIOError:
                continue
            if not chunk:
                return None
            self.buf += chunk
        return None

    def handshake(self, deadline: float) -> bool:
        if self.sock is None:
            return False
        greet = self._read_json(deadline)
        if not greet or "QMP" not in greet:
            return False
        self.sock.sendall(b'{"execute":"qmp_capabilities"}\n')
        reply = self._read_json(deadline)
        return bool(reply) and "return" in reply

    def inject_nmi(self, deadline: float) -> str:
        if self.sock is None:
            return "no-socket"
        try:
            self.sock.sendall(b'{"execute":"inject-nmi"}\n')
        except OSError as e:
            return f"send-error:{e}"
        reply = self._read_json(deadline)
        if reply is None:
            return "timeout"
        if "return" in reply:
            return "ok"
        err = reply.get("error") or {}
        desc = err.get("desc") or str(reply)
        return f"error:{desc}"

    def close(self) -> None:
        if self.sock is not None:
            try:
                self.sock.close()
            except OSError:
                pass
            self.sock = None


def main() -> int:
    if len(sys.argv) < 2:
        print("qemu-serial-inject: missing kernel ELF", file=sys.stderr)
        return 1
    elf = sys.argv[1]
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    img = prepare_fat16(root)

    qmp_dir = tempfile.mkdtemp(prefix="ctos-qmp-")
    qmp_sock = os.path.join(qmp_dir, "qmp.sock")
    qmp = QmpClient(qmp_sock)

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
        "-qmp",
        f"unix:{qmp_sock},server,nowait",
        "-drive",
        f"if=none,file={img},format=raw,id=hd0",
        "-device",
        "virtio-blk-device,drive=hd0",
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
    serror_armed = False
    serror_standing_seen = False
    serror_injected = False
    standing_count_at_arm = 0
    qmp_ready = False
    qmp_result = "not-attempted"

    # Connect QMP early (server,nowait) while guest boots.
    if qmp.connect(min(deadline, time.monotonic() + 2.0)):
        if qmp.handshake(min(deadline, time.monotonic() + 2.0)):
            qmp_ready = True
        else:
            qmp_result = "handshake-failed"
            qmp.close()
    else:
        qmp_result = "connect-failed"

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
                if not serror_armed and SERROR_ARM in buf:
                    serror_armed = True
                    standing_count_at_arm = buf.count(b"el0: standing")
                # Inject only after the SError probe's SVC #1 (`el0: standing`)
                # so PSTATE.A is already clear in the spin window.
                if (
                    serror_armed
                    and not serror_injected
                    and qmp_ready
                    and buf.count(b"el0: standing") > standing_count_at_arm
                ):
                    serror_standing_seen = True
                    time.sleep(SERROR_INJECT_DELAY)
                    qmp_result = qmp.inject_nmi(deadline)
                    serror_injected = True
                    msg = f"qemu-serial-inject: qmp inject-nmi => {qmp_result}\n"
                    sys.stdout.buffer.write(msg.encode())
                    sys.stdout.buffer.flush()
            elif proc.poll() is not None:
                break
        if proc.poll() is not None:
            rest = os.read(fd, 4096)
            if rest:
                sys.stdout.buffer.write(rest)
                sys.stdout.buffer.flush()
            break

    if serror_armed and not serror_injected:
        note = (
            f"qemu-serial-inject: serror-arm seen but qmp not injected "
            f"(ready={qmp_ready}, standing_after={serror_standing_seen}, "
            f"result={qmp_result})\n"
        )
        sys.stdout.buffer.write(note.encode())
        sys.stdout.buffer.flush()

    qmp.close()
    try:
        os.unlink(qmp_sock)
    except OSError:
        pass
    try:
        os.rmdir(qmp_dir)
    except OSError:
        pass

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
