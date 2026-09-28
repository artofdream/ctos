# Daily brief — 2026-09-27 — ADR-093 public-identifier scrub

- **What:** sponsor-approved DSO guardrail. The AWS account ID and related cloud resource identifiers (hosted zone, EC2 instances, Spot requests, security group, IAM user) plus a local OS user name in host paths were replaced with typed placeholders across 17 content files. Shell snippets now read `${AWS_ACCOUNT_ID}` / `${ROUTE53_ZONE_ID}` from local config.
- **Guard:** `scripts/check-public-ids.sh` is the first step of `qemu-smoke.sh` (CI + Docker). It has generic patterns only and never prints a matched value. The self-test plants synthetic tokens (7 classes). Unmodified `main` fails (77 raw hits); this branch passes.
- **Not done here:** git history still holds the old values. Cleanup (rewrite / force-push) is a separate sponsor decision after a risk evaluation. Open EL0 stack PRs are unchanged and will meet the guard after rebase.
- **Secrets:** none needed; no workflow reads these values.
- See [ADR-093](../../docs/03-adr/ADR-093-public-identifier-scrub-guard.md).
