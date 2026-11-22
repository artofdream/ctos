#!/usr/bin/env python3
"""Host-side reference for the ADR-114 R1 record-processing spec.

Derives the expected output (`workout`) from the golden input (`workin`)
exactly as docs/03-adr/ADR-114-r1-record-spec.md defines it. Pure python3
stdlib, read-only unless --write is given. This is a spec oracle, not the
EL0 app (that is R2) and not a measurement (no timing, no thresholds).

  python3 scripts/adr114-golden.py --check   # golden output matches input
  python3 scripts/adr114-golden.py --write   # regenerate workout from workin
  python3 scripts/adr114-golden.py --self-test
"""
import argparse
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FIX = ROOT / "tests" / "fixtures" / "adr114"
FILE_MAX = 1024  # vfs::FILE_MAX on main; input and output must fit
ID_MAX = 8
VALUE_DIGITS_MAX = 10
I32_MIN, I32_MAX = -(2**31), 2**31 - 1


class Fail(Exception):
    """A whole-run failure: `work: fail reason=<token>`; no complete output."""


def crc16_ccitt_false(data: bytes) -> int:
    """CRC-16/CCITT-FALSE: poly 0x1021, init 0xFFFF, no reflection, xorout 0."""
    crc = 0xFFFF
    for b in data:
        crc ^= b << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else (crc << 1)
            crc &= 0xFFFF
    return crc


def parse_record(line: bytes):
    """Return (id_bytes, value_int) for a valid record, else None (rejected)."""
    if line.count(b",") != 1:
        return None
    rid, val = line.split(b",")
    if not (1 <= len(rid) <= ID_MAX) or not all(0x30 <= c <= 0x39 for c in rid):
        return None
    digits = val[1:] if val[:1] == b"-" else val
    if not (1 <= len(digits) <= VALUE_DIGITS_MAX):
        return None
    if not all(0x30 <= c <= 0x39 for c in digits):
        return None
    n = int(digits.decode("ascii"))
    if val[:1] == b"-":
        n = -n
    if not (I32_MIN <= n <= I32_MAX):
        return None
    return rid, n


def process(data: bytes) -> bytes:
    if len(data) == 0:
        raise Fail("input_empty")
    if len(data) > FILE_MAX:
        raise Fail("input_too_big")
    lines = data.split(b"\n")
    tail = lines.pop()  # bytes after the last '\n' (empty if well terminated)
    out = bytearray()
    ok = rejected = 0
    for line in lines:
        rec = parse_record(line)
        if rec is None:
            rejected += 1
            continue
        rid, n = rec
        body = rid + b"," + str(n).encode("ascii")
        out += body + b"," + b"%04x" % crc16_ccitt_false(body) + b"\n"
        ok += 1
    if tail:
        rejected += 1  # truncated final line: never a valid record
    out += b"ok=%d rejected=%d\n" % (ok, rejected)
    if len(out) > FILE_MAX:
        raise Fail("output_too_big")
    return bytes(out)


def self_test() -> None:
    assert crc16_ccitt_false(b"123456789") == 0x29B1  # catalogue check value
    assert process(b"1,0\n") == b"1,0,%04x\nok=1 rejected=0\n" % crc16_ccitt_false(b"1,0")
    assert process(b"5,007\n6,-0\n").startswith(b"5,7,")
    assert process(b"\n").endswith(b"ok=0 rejected=1\n")
    assert process(b"1,2147483648\n2,-2147483649\n").endswith(b"ok=0 rejected=2\n")
    assert process(b"1,-2147483648\n").startswith(b"1,-2147483648,")
    assert process(b"1,5\r\n").endswith(b"ok=0 rejected=1\n")
    assert process(b"1,1\n2,2").endswith(b"ok=1 rejected=1\n")
    for bad, why in ((b"", "input_empty"), (b"1,0\n" * 300, "input_too_big"),
                     (b"1,0\n" * 200, "output_too_big")):
        try:
            process(bad)
        except Fail as e:
            assert str(e) == why, (why, e)
        else:
            raise AssertionError(why)


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--check", action="store_true")
    g.add_argument("--write", action="store_true")
    g.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    self_test()
    if a.self_test:
        print("adr114-golden: self-test ok")
        return 0
    data = (FIX / "workin").read_bytes()
    want = process(data)
    if a.write:
        (FIX / "workout").write_bytes(want)
        print(f"adr114-golden: wrote workout ({len(want)} bytes)")
        return 0
    have = (FIX / "workout").read_bytes()
    if have != want:
        print("adr114-golden: FAIL workout does not match workin under the spec")
        return 1
    print(f"adr114-golden: ok in={len(data)} out={len(have)} {want.splitlines()[-1].decode()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
