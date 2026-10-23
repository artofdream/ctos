#!/usr/bin/env python3
"""ADR-108 SError spike matrix (fail-closed).

Boots the stand-alone `vse-spike` image (research/serror-spike/vse) on a
given QEMU and checks, per row, that the expected serial line appears and
the forbidden one does not. Exit 1 on any mismatch.

  CTOS_QEMU=<qemu-system-aarch64> [CTOS_SPIKE_PIN=1] ./serror-matrix.py <elf>

Rows marked pin-only need the pinned TYPE_NMI build (ADR-083); on stock
QEMU the inject rows instead require `machine does not provide NMIs`.
"""
import json, os, re, socket, subprocess, sys, tempfile, time

QEMU = os.environ.get("CTOS_QEMU", "qemu-system-aarch64")
PIN = os.environ.get("CTOS_SPIKE_PIN") == "1"
TAKEN = r"serror taken vec=0x580 from=el0 ec=0x2f"

def qmp(sock_path):
    s = socket.socket(socket.AF_UNIX)
    for _ in range(100):
        try:
            s.connect(sock_path)
            break
        except OSError:
            time.sleep(0.05)
    f = s.makefile("rw")
    json.loads(f.readline())
    def cmd(name, **args):
        f.write(json.dumps({"execute": name, "arguments": args}) + "\n")
        f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r:
                return r
    cmd("qmp_capabilities")
    return cmd

def run(elf, mode, smp=1, inject_cpu=None, timeout=6.0):
    d = tempfile.mkdtemp()
    sock = os.path.join(d, "qmp")
    argv = [QEMU, "-M", "virt,virtualization=on", "-cpu", "cortex-a76",
            "-smp", str(smp), "-nographic", "-monitor", "none",
            "-serial", "stdio", "-qmp", f"unix:{sock},server=on,wait=off",
            "-kernel", elf, "-device",
            f"loader,addr=0x40200000,data={mode},data-len=1"]
    p = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    os.set_blocking(p.stdout.fileno(), False)
    out, note, cmd, t0 = b"", "", None, time.time()
    while time.time() - t0 < timeout:
        chunk = p.stdout.read() or b""
        out += chunk
        if cmd is None and os.path.exists(sock):
            cmd = qmp(sock)
        if inject_cpu is not None and cmd and b"inject now" in out and not note:
            time.sleep(0.2)
            r = cmd("human-monitor-command", **{"command-line": "nmi",
                                               "cpu-index": inject_cpu})
            note = "nmi=>" + (r.get("return") or "ok").strip() if "return" in r \
                else "nmi=>error " + r["error"]["desc"]
            if not r.get("return"):
                note = "nmi=>ok"
            t0 = time.time() - timeout + 1.5
        if b"vse: end" in out and inject_cpu is None:
            break
        time.sleep(0.05)
    p.kill()
    p.wait()
    out += p.stdout.read() or b""
    return out.decode(errors="replace").replace("\r", ""), note

ROWS = [
    # name, mode, smp, inject_cpu, expect, forbid, pin_only
    ("vse cpu0 taken",        0x00, 1, None, TAKEN + r".*", None, False),
    ("vse cpu0 no-VSE",       0x01, 1, None, r"none \(el0 reached svc\)", TAKEN, False),
    ("vse cpu0 A-masked",     0x02, 1, None, r"none \(el0 reached svc\)", TAKEN, False),
    ("vse cpu0 no-AMO",       0x03, 1, None, r"none \(el0 reached svc\)", TAKEN, False),
    ("vse cpu1 taken",        0x10, 2, None, r"cpu=0x1 " + TAKEN, None, False),
    ("vse cpu1 no-VSE",       0x11, 2, None, r"cpu=0x1 none", TAKEN, False),
    ("vse cpu1 A-masked",     0x12, 2, None, r"cpu=0x1 none", TAKEN, False),
    ("vse cpu1 no-AMO",       0x13, 2, None, r"cpu=0x1 none", TAKEN, False),
    ("bad PA = sync abort",   0x06, 1, None, r"sync abort \(not serror\) vec=0x400 ec=0x24 dfsc=0x10", TAKEN, False),
    ("nmi cpu0 el0",          0x04, 1, 0, r"cpu=0x0 " + TAKEN, None, True),
    ("nmi cpu0 el0 A-masked", 0x05, 1, 0, r"inject now", TAKEN, True),
    ("nmi cpu1 el0",          0x14, 2, 1, r"cpu=0x1 " + TAKEN, None, True),
    ("nmi to cpu0, cpu1 el0", 0x14, 2, 0, r"inject now", TAKEN, True),
]

def main():
    elf = sys.argv[1]
    ver = subprocess.run([QEMU, "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    print(f"serror-matrix: qemu={ver!r} pin={int(PIN)}")
    bad = 0
    for name, mode, smp, inj, expect, forbid, pin_only in ROWS:
        out, note = run(elf, mode, smp, inj)
        if pin_only and not PIN:
            ok = ("does not provide NMIs" in note) and not re.search(TAKEN, out)
            exp = "stock: nmi refused, no SError"
        else:
            ok = bool(re.search(expect, out)) and not (forbid and re.search(forbid, out))
            if pin_only:
                ok = ok and note == "nmi=>ok"
            exp = expect
        bad += not ok
        lines = [l for l in out.splitlines() if l.startswith("vse: cpu=")]
        print(f"serror-matrix: {'ok  ' if ok else 'FAIL'} {name:24s} mode={mode:#04x} smp={smp}"
              f" {note} | {lines[-1] if lines else '(no vector line)'}")
        if not ok:
            print(f"serror-matrix:   expected {exp!r}; serial:\n{out}")
    print(f"serror-matrix: {'ok' if not bad else 'FAILED'} rows={len(ROWS)} bad={bad}")
    sys.exit(1 if bad else 0)

if __name__ == "__main__":
    main()
