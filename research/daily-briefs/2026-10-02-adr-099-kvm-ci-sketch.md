# 2026-10-02 — ADR-099: KVM re-check in CI, design sketch

Milestone item 2, design only. Draft PR; no AWS call, no AWS resource created or changed; nothing merged by this work; PRs #149 and #150 untouched (#151 was merged by the sponsor at 13:01 CEST).

- **Deliverables:** [ADR-099](../../docs/03-adr/ADR-099-kvm-recheck-ci-oidc-sketch.md), an inert [`kvm-recheck.yml`](../../.github/workflows/kvm-recheck.yml), and `scripts/kvm-recheck-userdata.sh`.
- **Sponsor exception (PX0, DSO relay 2026-09-28 21:23 CEST):** an OIDC role with no stored keys, conditional on (a) CloudTrail on, (b) branch protection enforced for admins, (c) results-out decided before any extra permission. The DSO vault record was not readable by the executor and was not read.
- **Results-out recommendation:** serial console via `ec2:GetConsoleOutput`, tag-scoped. No `iam:PassRole`, no instance profile, no S3, no SSM, no credential on the instance.
- **Checked read-only (GitHub, today):** enforce-admins on; no approving review required; no KVM environment yet; no `KVM_*` variable (workflow inert); the OIDC `sub` uses the immutable-subject form, so an example trust policy would not match.
- **Not verified:** CloudTrail, OIDC provider, Spot service-linked role and quota, EBS default key, exact `sub` string, console readability live vs after termination, IAM condition behaviour, budget alarm.
- **Cost:** about $0.03–0.07 per run [estimate]; computed worst case about $0.33 per run [estimate].
