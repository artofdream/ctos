"""Self-test for check-archify-honesty.py: one passing and one failing synthetic IR,
plus an end-to-end run against a scratch directory. Standard library only; runs under
`python3 scripts/test_check_archify_honesty.py` or pytest (which collects the TestCase)."""

import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check-archify-honesty.py"
spec = importlib.util.spec_from_file_location("check_archify_honesty", SCRIPT)
guard = importlib.util.module_from_spec(spec)
sys.modules["check_archify_honesty"] = guard
spec.loader.exec_module(guard)

CFG = {**guard.DEFAULTS, "phrases": ["Not FEAT_NMI"]}

GOOD = {
    "schema_version": 1,
    "diagram_type": "architecture",
    "meta": {"title": "Good", "output": "good.architecture.html"},
    "components": [
        {"id": "a", "type": "backend", "label": "Service", "sublabel": "Simulated", "pos": [0, 0], "size": [100, 50]},
        {"id": "b", "type": "external", "label": "Probe", "tag": "Verified", "pos": [200, 0], "size": [100, 50]},
    ],
    "connections": [],
    "cards": [{"dot": "emerald", "title": "Boundary", "items": ["Not FEAT_NMI.", "Gap: no hosted instance"]}],
}


def bad():
    ir = copy.deepcopy(GOOD)
    ir["meta"]["views"] = [{"id": "v1", "label": "Honesty", "focus": ["a"], "note": "Not FEAT_NMI. Not hosted. Probed once."}]
    ir["cards"] = [{"dot": "emerald", "title": "Boundary", "items": ["Nothing about status here"]}]
    ir["components"][1]["tag"] = "edge"
    return ir


def html(gen):
    return f'<html><head><meta name="generator" content="{gen}"></head></html>'


class ArchifyHonestyGuard(unittest.TestCase):
    def test_passing_ir(self):
        r = guard.check_ir(copy.deepcopy(GOOD), CFG, file="good.json", html=html("archify 3.2.0"))
        self.assertEqual(r["errors"], [])
        self.assertEqual(r["warnings"], [])

    def test_failing_ir_reports_views_and_note_only_wording(self):
        r = guard.check_ir(bad(), CFG, file="bad.json", html=html("archify 3.2.0"))
        self.assertTrue(any("meta.views exists" in e for e in r["errors"]))
        for term in ("Not FEAT_NMI", "Not hosted", "Probed"):
            self.assertTrue(any(f'"{term}"' in e for e in r["errors"]), term)

    def test_wording_moved_into_cards_only_trips_the_views_rule(self):
        ir = bad()
        ir["cards"] = [{"dot": "emerald", "title": "Boundary", "items": ["Not FEAT_NMI. Not hosted. Probed once."]}]
        r = guard.check_ir(ir, CFG, file="moved.json")
        self.assertEqual(len(r["errors"]), 1)
        self.assertIn("meta.views exists", r["errors"][0])

    def test_word_boundaries(self):
        ir = copy.deepcopy(GOOD)
        ir["meta"]["views"] = [{"id": "v", "label": "x", "focus": ["a"], "note": "Gap"}]
        ir["cards"] = [{"dot": "emerald", "title": "t", "items": ["Gaps remain"]}]
        r = guard.check_ir(ir, CFG, file="wb.json")
        self.assertTrue(any('"Gap"' in e for e in r["errors"]))

    def test_generator_warning_is_non_failing_and_timeline_is_exempt(self):
        r = guard.check_ir(copy.deepcopy(GOOD), CFG, file="g.json", html=html("archify 3.1.0"))
        self.assertEqual(r["errors"], [])
        self.assertEqual(len(r["warnings"]), 1)
        self.assertEqual(guard.generators_of(html("archify 3.2.0")), ["archify 3.2.0"])
        # a wrapper page that embeds the real page entity-escaped (iframe srcdoc)
        wrapped = '<iframe srcdoc="&lt;meta name=&quot;generator&quot; content=&quot;archify 3.2.0&quot;&gt;"></iframe>'
        self.assertEqual(guard.generators_of(wrapped), ["archify 3.2.0"])
        self.assertEqual(guard.check_ir(copy.deepcopy(GOOD), CFG, file="w.json", html=wrapped)["warnings"], [])
        t = copy.deepcopy(GOOD)
        t["diagram_type"] = "timeline"
        self.assertEqual(guard.check_ir(t, CFG, file="t.json", html="<html></html>")["warnings"], [])

    def test_cli_exit_codes(self):
        with tempfile.TemporaryDirectory() as tmp:
            docs = Path(tmp) / "docs"
            docs.mkdir()
            (docs / "good.architecture.json").write_text(json.dumps(GOOD))
            (docs / "good.architecture.html").write_text(html("archify 3.2.0"))
            cmd = [sys.executable, str(SCRIPT), "--root", tmp]
            ok = subprocess.run(cmd, capture_output=True, text=True)
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)

            b = bad()
            b["meta"]["output"] = "bad.architecture.html"
            (docs / "bad.architecture.json").write_text(json.dumps(b))
            (docs / "bad.architecture.html").write_text(html("archify 2.17.0-dev.1"))
            ko = subprocess.run(cmd, capture_output=True, text=True)
            self.assertEqual(ko.returncode, 1, ko.stdout + ko.stderr)
            self.assertIn("meta.views exists", ko.stderr)
            self.assertIn('generator is "archify 2.17.0-dev.1"', ko.stderr)


if __name__ == "__main__":
    unittest.main()
