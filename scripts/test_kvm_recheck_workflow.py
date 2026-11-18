"""Self-test for kvm-recheck.yml's guard and launch loop (ADR-112 rev 2026-11-18).

Runs the real `Guard` and `Launch` step scripts, cut out of the workflow file,
under bash with a fake `aws` on PATH. No AWS call, no network, no credentials.
Needs PyYAML, bash and openssl. A pass here says the guard refuses types outside
the allowlist and the launch loop walks c7g.metal -> m7g.metal -> m6g.metal across
the AZs, moving on only on capacity errors. It says nothing about real EC2
capacity or about what KVM delivers on any instance type."""

import os
import re
import shutil
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent
WF = ROOT / ".github" / "workflows" / "kvm-recheck.yml"
DOC = yaml.safe_load(WF.read_text())
JOB = DOC["jobs"]["kvm-recheck"]
STEPS = {s["name"]: s for s in JOB["steps"]}


def step(prefix):
    (name,) = [n for n in STEPS if n.startswith(prefix)]
    return STEPS[name]["run"]


# Placeholder values only (ADR-093: no real identifiers).
FAKE_ENV = {
    "GITHUB_REPOSITORY": "artofdream/ctos",
    "GITHUB_REF": "refs/heads/main",
    "GITHUB_RUN_ID": "1",
    "KVM_ROLE_ARN": "arn:aws:iam::123456789012:role/example",
    "KVM_AMI_ID": "ami-placeholder",
    "KVM_SG_ID": "sg-1234567890abcdef0",
    "KVM_SUBNET_IDS": "subnet-a subnet-b subnet-c",
    "INPUT_REF": "main",
    "PILOT_ACK": "supervised-b3-run",
}

# The fake aws: answers run-instances from a script of per-(type) outcomes and
# logs every attempt as "<type> <subnet>".
FAKE_AWS = r"""#!/usr/bin/env bash
itype=""; subnet=""
while [ $# -gt 0 ]; do
  case "$1" in
    --instance-type) itype="$2"; shift ;;
    --subnet-id) subnet="$2"; shift ;;
  esac
  shift
done
echo "$itype $subnet" >> "$FAKE_LOG"
outcome=$(sed -n "s/^$itype=//p" "$FAKE_PLAN" | head -1)
case "$outcome" in
  ok) echo "i-1234567890abcdef0"; exit 0 ;;
  "") outcome=InsufficientInstanceCapacity ;;
esac
echo "An error occurred ($outcome) when calling the RunInstances operation: x" >&2
exit 254
"""


def sub_expr(script):
    # GitHub expressions in the launch step: only the commit sha is used.
    return re.sub(r"\$\{\{[^}]*\}\}", "0" * 40, script)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        (self.tmp / "bin").mkdir()
        aws = self.tmp / "bin" / "aws"
        aws.write_text(FAKE_AWS)
        aws.chmod(aws.stat().st_mode | stat.S_IEXEC)
        (self.tmp / "scripts").mkdir()
        shutil.copy(ROOT / "scripts" / "kvm-recheck-userdata.sh", self.tmp / "scripts")
        self.summary = self.tmp / "summary.md"
        self.output = self.tmp / "output.env"

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def env(self, **over):
        e = dict(os.environ)
        e.update({k: str(v) for k, v in JOB["env"].items() if "${{" not in str(v)})
        e.update(FAKE_ENV)
        e.update(
            PATH=f"{self.tmp / 'bin'}:{os.environ['PATH']}",
            GITHUB_STEP_SUMMARY=str(self.summary),
            GITHUB_OUTPUT=str(self.output),
            FAKE_LOG=str(self.tmp / "aws.log"),
            FAKE_PLAN=str(self.tmp / "plan"),
        )
        e.update(over)
        return e

    def run_step(self, prefix, plan="", **over):
        (self.tmp / "plan").write_text(plan)
        return subprocess.run(
            ["bash", "-c", sub_expr(step(prefix))],
            cwd=self.tmp, env=self.env(**over), capture_output=True, text=True,
        )

    def attempts(self):
        p = self.tmp / "aws.log"
        return p.read_text().split("\n")[:-1] if p.exists() else []

    def result(self):
        p = self.tmp / "kvm-result.env"
        return p.read_text() if p.exists() else ""


class Guard(Base):
    def test_default_list_passes(self):
        r = self.run_step("Guard")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("Instance types, in order: c7g.metal m7g.metal m6g.metal", r.stdout)

    def test_default_order_is_pinned(self):
        self.assertEqual(JOB["env"]["INSTANCE_TYPES"], "c7g.metal m7g.metal m6g.metal")

    def test_refuses_types_outside_allowlist(self):
        for bad in ("c6g.metal", "c7g.large", "c8g.metal-24xl", "m7g.metal c6g.metal",
                    "c7g.metal,r7g.metal", "x86.metal", "c7g.metal c7g.metal", ""):
            with self.subTest(types=bad):
                r = self.run_step("Guard", INSTANCE_TYPES=bad)
                self.assertNotEqual(r.returncode, 0, bad)
                # Refused for the type list, not for some other guard line.
                self.assertRegex(r.stdout, r"refused: (instance type|INSTANCE_TYPES)", bad)
                self.assertIn("NOT-RUN", self.summary.read_text())
        self.assertEqual(self.attempts(), [])

    def test_refuses_without_ack(self):
        r = self.run_step("Guard", PILOT_ACK="")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("acknowledgement", r.stdout)

    def test_comma_separated_allowed(self):
        r = self.run_step("Guard", INSTANCE_TYPES="m6g.metal,c7g.metal")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_other_layers_unchanged(self):
        env = JOB["env"]
        self.assertEqual(env["MAX_PRICE_USD_HR"], "0.40")
        self.assertEqual(env["LIFETIME_MIN"], "45")
        self.assertEqual(env["RUN_COST_CAP_USD"], "0.50")
        self.assertEqual(env["COMPONENT_TAG"], "b2-spike")
        self.assertEqual(env["RUN_ACK"], "supervised-b3-run")
        self.assertEqual(JOB["environment"], "kvm-pilot")
        launch = step("Launch")
        for needle in ("SpotInstanceType=one-time", "MaxPrice=$MAX_PRICE_USD_HR",
                       "--instance-initiated-shutdown-behavior terminate",
                       "Key=component,Value=$COMPONENT_TAG"):
            self.assertIn(needle, launch)
        for name in ("Terminate and sweep", "Verdict"):
            (n,) = [s for s in STEPS if s.startswith(name)]
            self.assertEqual(STEPS[n]["if"].split(" ")[0], "always()")


class Launch(Base):
    def test_first_type_first_az(self):
        r = self.run_step("Launch", "c7g.metal=ok\n")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.attempts(), ["c7g.metal subnet-a"])
        self.assertIn("instance_type=c7g.metal", self.output.read_text())

    def test_falls_back_to_m7g_after_all_c7g_azs(self):
        r = self.run_step("Launch", "m7g.metal=ok\n")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(self.attempts(), [
            "c7g.metal subnet-a", "c7g.metal subnet-b", "c7g.metal subnet-c",
            "m7g.metal subnet-a"])
        self.assertIn("instance_type=m7g.metal", self.output.read_text())

    def test_falls_back_to_m6g(self):
        r = self.run_step("Launch", "m7g.metal=Unsupported\nm6g.metal=ok\n")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual(len(self.attempts()), 7)
        self.assertIn("instance_type=m6g.metal", self.output.read_text())

    def test_no_capacity_anywhere_is_skipped(self):
        r = self.run_step("Launch", "")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(len(self.attempts()), 9)
        res = self.result()
        self.assertIn("result=SKIPPED-no-capacity", res)
        self.assertIn("reason=no-capacity-in-9-attempts", res)
        self.assertIn("instance_type=none (tried: c7g.metal m7g.metal m6g.metal)", res)

    def test_non_capacity_error_fails_immediately(self):
        # e.g. the role not yet allowing m7g.metal: FAIL at once, no further tries.
        r = self.run_step("Launch", "m7g.metal=UnauthorizedOperation\nm6g.metal=ok\n")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.attempts()[-1], "m7g.metal subnet-a")
        self.assertEqual(len(self.attempts()), 4)
        res = self.result()
        self.assertIn("result=FAIL", res)
        self.assertIn("reason=launch-error-UnauthorizedOperation", res)

    def test_quota_error_skips_immediately(self):
        r = self.run_step("Launch", "c7g.metal=VcpuLimitExceeded\n")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(len(self.attempts()), 1)
        self.assertIn("reason=quota-or-limit", self.result())


class Verdict(unittest.TestCase):
    def test_verdict_and_wait_name_the_type(self):
        self.assertIn("instance_type", step("Verdict"))
        wait = step("Wait")
        self.assertIn("trap record_type EXIT", wait)
        self.assertIn("kvm-cap-serror-esr", wait)  # PASS still needs the cap


if __name__ == "__main__":
    unittest.main()
