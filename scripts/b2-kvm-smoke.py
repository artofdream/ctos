#!/usr/bin/env python3
"""ADR-085 B2-P: fail-closed taken-SError smoke under QEMU -accel kvm.

Boots the `b2-serror` ctos ELF on a KVM-capable arm64 host (real Graviton
.metal) with the ADR-085 QEMU (0001+0002). On the guest cue
`el0: serror-arm` it issues QMP stop -> inject-nmi -> cont. Under KVM the
0002 patch turns inject-nmi into KVM_SET_VCPU_EVENTS{serror_pending=1}
(architected virtual SError, HCR_EL2.VSE).

FAIL-CLOSED: exit 0 ONLY if a bare `el0: serror` line (the lower-EL
SError handler's taken marker) appears AND `b2: taken`. Absence of the
marker, `el0: serror-park`, `b2: not taken`, a missing KVM accelerator, or a
timeout all exit non-zero. Never passes on absence.
"""
from __future__ import annotations

import json, os, re, select, socket, subprocess, sys, tempfile, time

QEMU = os.environ.get("CTOS_QEMU", "qemu-system-aarch64")
TIMEOUT = float(os.environ.get("CTOS_B2_TIMEOUT", "90"))
# Diagnostics only (never a pass condition): if the guest is silent this
# long, dump vCPU registers via QMP human-monitor-command and resolve PC.
STALL = float(os.environ.get("CTOS_B2_STALL", "15"))
# Box self-test only: CTOS_B2_ACCEL=tcg runs the same flow under TCG
# (-cpu max). The B2 claim is only ever made from a kvm run.
ACCEL = os.environ.get("CTOS_B2_ACCEL", "kvm")
ARM = b"el0: serror-arm"
# Bare taken marker: a line that is exactly `el0: serror` (not -arm/-park).
TAKEN_RE = re.compile(rb"(?m)^el0: serror\r?$")


def log(msg: str) -> None:
    sys.stdout.write(f"b2-kvm-smoke: {msg}\n")
    sys.stdout.flush()


class Qmp:
    def __init__(self, path: str) -> None:
        self.path, self.sock, self.buf = path, None, b""

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

    def _read(self, deadline: float):
        while time.monotonic() < deadline:
            if b"\n" in self.buf:
                line, self.buf = self.buf.split(b"\n", 1)
                if line.strip():
                    return json.loads(line.decode())
                continue
            r, _, _ = select.select([self.sock], [], [], 0.05)
            if r:
                try:
                    c = self.sock.recv(4096)
                except OSError:
                    return None
                if not c:
                    return None
                self.buf += c
        return None

    def cmd(self, name: str, deadline: float) -> str:
        try:
            self.sock.sendall(json.dumps({"execute": name}).encode() + b"\n")
        except OSError as e:
            return f"send-error:{e}"
        while True:
            r = self._read(deadline)
            if r is None:
                return "timeout"
            if "event" in r and "return" not in r and "error" not in r:
                continue
            if "return" in r:
                return "ok"
            return "error:" + ((r.get("error") or {}).get("desc") or str(r))

    def hmp(self, line: str, deadline: float) -> str:
        """QMP human-monitor-command; returns the HMP text (diagnostics)."""
        try:
            self.sock.sendall(json.dumps({"execute": "human-monitor-command",
                                          "arguments": {"command-line": line}}).encode() + b"\n")
        except OSError as e:
            return f"send-error:{e}"
        while True:
            r = self._read(deadline)
            if r is None:
                return "timeout"
            if "event" in r and "return" not in r and "error" not in r:
                continue
            if "return" in r:
                return str(r["return"])
            return "error:" + str(r.get("error"))

    def handshake(self, deadline: float) -> bool:
        g = self._read(deadline)
        if not g or "QMP" not in g:
            return False
        return self.cmd("qmp_capabilities", deadline) == "ok"


def dump_stall(q: "Qmp", elf: str, why: str) -> None:
    """Print PC/SP/PSTATE (+ addr2line) twice, 1 s apart. Diagnostics only."""
    log(f"stall: {why}; dumping vCPU state (diagnostics, not a pass signal)")
    for n in range(2):
        txt = q.hmp("info registers", time.monotonic() + 5)
        keep = [l for l in txt.splitlines() if re.search(r"\b(PC|SP|PSTATE|X0[0-3]|X30)\b|PC=|PSTATE=", l)]
        for l in keep[:8]:
            log(f"regs[{n}] {l.strip()}")
        m = re.search(r"PC=([0-9a-fA-F]+)", txt)
        if m:
            pc = m.group(1)
            try:
                a2l = subprocess.run(["addr2line", "-f", "-C", "-e", elf, "0x" + pc],
                                     capture_output=True, text=True, timeout=10).stdout
                log(f"regs[{n}] addr2line 0x{pc}: " + " | ".join(a2l.split("\n")[:2]))
            except Exception as e:  # noqa: BLE001
                log(f"regs[{n}] addr2line unavailable: {e}")
        time.sleep(1)


def main() -> int:
    if len(sys.argv) < 2:
        log("FAIL missing b2-serror kernel ELF")
        return 2
    elf = sys.argv[1]
    if ACCEL == "kvm" and not os.path.exists("/dev/kvm"):
        log("FAIL /dev/kvm absent (need a KVM-capable arm64 host, e.g. Graviton .metal)")
        return 3
    d = tempfile.mkdtemp(prefix="ctos-b2-")
    qs = os.path.join(d, "qmp.sock")
    cmd = [QEMU, "-accel", ACCEL, "-machine", "virt,gic-version=3",
           "-cpu", "host" if ACCEL == "kvm" else "max",
           "-m", "128M", "-display", "none", "-serial", "stdio", "-monitor", "none",
           "-qmp", f"unix:{qs},server,nowait", "-kernel", elf]
    log("cmd: " + " ".join(cmd))
    p = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT)
    fd = p.stdout.fileno()
    os.set_blocking(fd, False)
    q = Qmp(qs)
    deadline = time.monotonic() + TIMEOUT
    qmp_ok = q.connect(min(deadline, time.monotonic() + 5)) and \
        q.handshake(min(deadline, time.monotonic() + 5))
    log(f"qmp ready={qmp_ok}")
    buf = bytearray()
    injected = False
    last_out = time.monotonic()
    dumped = False
    while time.monotonic() < deadline:
        if qmp_ok and not dumped and time.monotonic() - last_out > STALL:
            dump_stall(q, elf, f"no guest output for {STALL:.0f}s (bytes so far={len(buf)})")
            dumped = True
            break
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            c = os.read(fd, 4096)
            if c:
                last_out = time.monotonic()
                sys.stdout.buffer.write(c)
                sys.stdout.buffer.flush()
                buf += c
                if not injected and ARM in buf and qmp_ok:
                    s1 = q.cmd("stop", deadline)
                    s2 = q.cmd("inject-nmi", deadline)
                    s3 = q.cmd("cont", deadline)
                    injected = True
                    log(f"qmp stop=>{s1} inject-nmi=>{s2} cont=>{s3} "
                        f"(ADR-085: KVM_SET_VCPU_EVENTS serror_pending)")
                if b"b2: done" in buf:
                    break
            elif p.poll() is not None:
                break
        if p.poll() is not None:
            break
    if p.poll() is None:
        p.kill()
        try:
            p.wait(timeout=2)
        except subprocess.TimeoutExpired:
            p.terminate()
    text = bytes(buf).replace(b"\r", b"")
    taken = bool(TAKEN_RE.search(text)) and b"b2: taken" in text
    park = b"el0: serror-park" in text or b"b2: not taken" in text
    kvm_err = b"failed to initialize kvm" in text or b"invalid accelerator" in text
    log(f"summary armed={ARM in text} injected={injected} "
        f"taken_marker={bool(TAKEN_RE.search(text))} b2_taken={b'b2: taken' in text} "
        f"park={park} kvm_err={kvm_err}")
    if taken and not park:
        log(f"PASS taken lower-EL SError observed under accel={ACCEL} ('el0: serror' + 'b2: taken')")
        return 0
    log("FAIL taken 'el0: serror' NOT observed (fail-closed; never pass on absence)")
    return 1


if __name__ == "__main__":
    sys.exit(main())
