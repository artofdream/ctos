# 2026-10-02 — ADR-099: KVM re-check in CI, design sketch

Milestone item 2, design only. Draft PR; no AWS call, no AWS resource created or changed; nothing merged by this work; PRs #149 and #150 untouched (#151 was merged by DSO on the sponsor's instruction at 13:01 CEST).

- **Deliverables:** [ADR-099](../../docs/03-adr/ADR-099-kvm-recheck-ci-oidc-sketch.md), an inert [`kvm-recheck.yml`](../../.github/workflows/kvm-recheck.yml), and `scripts/kvm-recheck-userdata.sh`.
- **Sponsor exception (PX0, DSO relay 2026-09-28 21:23 CEST):** an OIDC role with no stored keys, conditional on (a) CloudTrail on, (b) branch protection enforced for admins, (c) results-out decided before any extra permission. The DSO vault record was not readable by the executor and was not read.
- **Results-out recommendation:** serial console via `ec2:GetConsoleOutput`, tag-scoped. No `iam:PassRole`, no instance profile, no S3, no SSM, no credential on the instance.
- **Checked read-only (GitHub, today):** enforce-admins on; no approving review required; no KVM environment yet; no `KVM_*` variable (workflow inert); the OIDC `sub` uses the immutable-subject form, so an example trust policy would not match.
- **Not verified:** CloudTrail, OIDC provider, Spot service-linked role and quota, EBS default key, exact `sub` string, console readability live vs after termination, IAM condition behaviour, budget alarm.
- **Cost:** about $0.03–0.07 per run [estimate]; computed worst case about $0.33 per run [estimate].
- **Update 2026-10-02 (DSO):** the sponsor accepted section 2 (results out via tag-scoped `ec2:GetConsoleOutput`, no PassRole, no instance profile) in writing, recorded in the DSO vault. ADR-099 now lists the open reconciliation points (immutable-subject string, environment first, SHA-pinned actions, region `eu-north-1`). The PR stays a draft.

## Update 2026-10-13: one-off supervised pilot spec

- Sponsor decision (20:49 CEST, DSO relay): #152 becomes the exact spec for **one** supervised pilot run; stays draft; manual dispatch only, no schedule.
- Environment `kvm-pilot` (required reviewers, main-only deployment branches) to be created by DSO **before** the role; not created by the agent.
- Actions pinned by full SHA (checkout v7.0.1, configure-aws-credentials v6.3.0); region `eu-north-1`; tags `project=ctos`, `component=kvm-ci-pilot`.
- Spot price cap is not expressible as an IAM condition; enforced by the workflow guard and a budget alarm.
- CloudTrail (P1) met: DSO verified a multi-region trail, home region `eu-north-1`, from the arm64 workstation (2026-10-13 about 20:56 CEST). No AWS call by the agent.
