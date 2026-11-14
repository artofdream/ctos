"""Self-test for b2-kvm-smoke.py's fail-closed verdict (ADR-112 B3 rows N, D, E).

Synthetic guest text only; no QEMU, no KVM. Standard library only; runs under
`python3 scripts/test_b2_kvm_smoke.py` or pytest. A pass here says the parser is
fail-closed. It says nothing about what real KVM delivers."""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "b2-kvm-smoke.py"
spec = importlib.util.spec_from_file_location("b2_kvm_smoke", SCRIPT)
smoke = importlib.util.module_from_spec(spec)
sys.modules["b2_kvm_smoke"] = smoke
spec.loader.exec_module(smoke)

BOOT = b"b2: e0 entry\nb2: boot (ADR-085 KVM SError profile)\nel0: serror-arm\n"
TAKEN = BOOT + b"el0: serror\nb2: taken\nb2: esr=0xbf0000c7\nb2: done\n"
PARK = BOOT + b"el0: serror-park\nb2: not taken\nb2: done\n"
E = "0xbf0000c7"


def ev(text, injected=True, ok=True, no_inject=False, expect=""):
    return smoke.evaluate(text, injected, ok, no_inject, expect, "kvm")


class Rows(unittest.TestCase):
    def test_row_e_exact_match_passes(self):
        rc, msg = ev(TAKEN, expect=E)
        self.assertEqual(rc, 0, msg)
        self.assertIn("PASS taken lower-EL SError observed under accel=kvm", msg)
        self.assertIn("esr=0xbf0000c7 match=exact", msg)

    def test_row_e_wrong_value_fails(self):
        rc, msg = ev(TAKEN, expect="0xbf0000c8")
        self.assertEqual(rc, 1)
        self.assertIn("esr-mismatch", msg)

    def test_row_e_default_esr_fails(self):
        # The syndrome was dropped (has_esr ignored): KVM default ISS only.
        rc, msg = ev(TAKEN.replace(b"0xbf0000c7", b"0xbf000000"), expect=E)
        self.assertEqual(rc, 1)
        self.assertIn("esr-mismatch", msg)

    def test_row_e_missing_esr_line_fails(self):
        rc, msg = ev(TAKEN.replace(b"b2: esr=0xbf0000c7\n", b""), expect=E)
        self.assertEqual(rc, 1)
        self.assertIn("observed=none", msg)

    def test_row_e_two_esr_lines_fail(self):
        rc, _ = ev(TAKEN.replace(b"b2: done", b"b2: esr=0xbf0000c7\nb2: done"), expect=E)
        self.assertEqual(rc, 1)

    def test_row_e_prefix_is_not_a_match(self):
        rc, _ = ev(TAKEN.replace(b"0xbf0000c7", b"0xbf0000c71"), expect=E)
        self.assertEqual(rc, 1)

    def test_upper_case_line_is_not_a_match(self):
        rc, _ = ev(TAKEN.replace(b"0xbf0000c7", b"0xBF0000C7"), expect=E)
        self.assertEqual(rc, 1)

    def test_row_d_default_esr(self):
        rc, msg = ev(TAKEN.replace(b"0xbf0000c7", b"0xbf000000"), expect="0xbf000000")
        self.assertEqual(rc, 0, msg)

    def test_inject_error_fails(self):
        # Patch 0003 refuses (no KVM_CAP_ARM_INJECT_SERROR_ESR): never a pass.
        rc, msg = ev(PARK, ok=False, expect=E)
        self.assertEqual(rc, 1)
        self.assertIn("inject-error", msg)

    def test_taken_plus_park_fails(self):
        rc, _ = ev(TAKEN.replace(b"b2: done", b"el0: serror-park\nb2: done"), expect=E)
        self.assertEqual(rc, 1)

    def test_armed_only_marker_is_not_taken(self):
        rc, _ = ev(BOOT + b"b2: taken\nb2: esr=0xbf0000c7\n", expect=E)
        self.assertEqual(rc, 1)

    def test_legacy_no_expect_still_passes(self):
        rc, msg = ev(TAKEN)
        self.assertEqual(rc, 0, msg)

    def test_absence_fails(self):
        rc, _ = ev(BOOT)
        self.assertEqual(rc, 1)


class RowN(unittest.TestCase):
    def test_row_n_park_passes(self):
        rc, msg = ev(PARK, injected=False, ok=False, no_inject=True)
        self.assertEqual(rc, 0, msg)
        self.assertIn("PASS no-inject row parked", msg)

    def test_row_n_taken_fails(self):
        rc, _ = ev(TAKEN, injected=False, ok=False, no_inject=True)
        self.assertEqual(rc, 1)

    def test_row_n_inject_sent_fails(self):
        rc, _ = ev(PARK, injected=True, ok=True, no_inject=True)
        self.assertEqual(rc, 1)

    def test_row_n_needs_done(self):
        rc, _ = ev(PARK.replace(b"b2: done\n", b""), injected=False, ok=False, no_inject=True)
        self.assertEqual(rc, 1)

    def test_row_n_half_park_fails(self):
        rc, _ = ev(BOOT + b"el0: serror-park\nb2: done\n", injected=False, ok=False, no_inject=True)
        self.assertEqual(rc, 1)

    def test_row_n_esr_line_fails(self):
        rc, _ = ev(PARK.replace(b"b2: done", b"b2: esr=0xbf000000\nb2: done"),
                   injected=False, ok=False, no_inject=True)
        self.assertEqual(rc, 1)


if __name__ == "__main__":
    unittest.main(verbosity=1)
