# 2026-11-18 — ADR-112 B3: instance-type fallback (option B), not run

- Why: kvm-recheck [run 43598194390](https://github.com/artofdream/ctos/actions/runs/43598194390) and [run 44227097081](https://github.com/artofdream/ctos/actions/runs/44227097081) were both SKIPPED-no-capacity (`c7g.metal` Spot, all three eu-north-1 AZs, $0, sweep clean). The user chose option B on 2026-11-18 at 04:16 CET.
- What: `kvm-recheck.yml` tries `c7g.metal` → `m7g.metal` → `m6g.metal`, each across every AZ. The guard refuses types outside that allowlist or not `*.metal`. The verdict and summary record `instance_type=`. `c6g.metal` is excluded (interruption rate) and On-Demand is not added. MaxPrice 0.40, LIFETIME 45, cap 0.50 and teardown are unchanged.
- Local probes (amd64, throwaway clone): `scripts/test_kvm_recheck_workflow.py` (the real Guard and Launch scripts against a fake `aws`), `scripts/test_b2_kvm_smoke.py`, actionlint with shellcheck, and the public-identifier guard.
- Before the next dispatch: the sponsor applies the IAM instance-type change (allow all three types) and DSO re-reads it. Without it, a fallback launch is FAIL (`UnauthorizedOperation`), never a pass.
- Still a **virtual SError**. No AWS call, no dispatch. Ledger row stays Planned. See [ADR-112](../../docs/03-adr/ADR-112-b3-graviton-real-hardware-serror-plan.md#revision-2026-11-18-instance-type-fallback-option-b).
