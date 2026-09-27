# ADR-093 — Public-identifier scrub + fail-closed guard

- Status: Accepted (security / compliance guardrail). Not a kernel change. No `src/` change.
- Date: 2026-09-27
- Trigger: sponsor-approved task relayed by DSO (2026-09-27): remove the main AWS account ID and other identifying data from the current tree of this public repo; add a cheap fail-closed guard if it fits.
- Base: branched from `main` (`c1d98c7`), **not** stacked on the open EL0 stack (#139–#144).
- Numbering: ADR-087–092 are taken by the open EL0 stack; this ADR takes 093 to avoid a collision on rebase.

## Context

This repository is public. A read-only scan found a cloud account identifier and related cloud resource identifiers written into docs, research notes and one Route 53 recovery batch file. Most came from the 2026-09-11 docs/DNS session and the 2026-09-26 B2-P paid spike ([ADR-085](ADR-085-b2p-graviton-kvm-serror.md)). A second scan found a local OS user name inside host paths and a WSL command line. No workflow, script logic or kernel code uses any of these values.

## Decision

1. **Replace, don't delete.** Each value becomes a typed placeholder so the prose keeps its meaning:
   - prose / tables: `<aws-account-id>`, `<route53-zone-id>`, `<instance-id-run0>` … `<instance-id-run4>`, `<spot-request-id-run0>` … `<spot-request-id-run4>`, `<security-group-id>`, `<iam-user>`, `<user>` (local OS user in paths). Run indices keep the ADR-085 tables cross-referenced.
   - copy-paste shell snippets: `"${AWS_ACCOUNT_ID:?set AWS_ACCOUNT_ID locally}"` and `"${ROUTE53_ZONE_ID:?set ROUTE53_ZONE_ID locally}"`. The real values stay in the operator's local config. Nothing in CI needs them, so **no new secret or variable** is needed.
2. **Counts (types only, no values).** Current tree on `main`:

   | Identifier type | Files | Occurrences | Where |
   | --- | --- | --- | --- |
   | AWS account ID | 11 | 14 | 4 docs (incl. README), 6 research notes, 1 script data file (Route 53 JSON comment); 0 workflows |
   | Route 53 hosted-zone ID | 12 | 17 | 5 docs, 6 research notes, 1 script data file |
   | EC2 instance IDs (5 distinct) | 4 | 27 | 2 docs, 2 research notes |
   | Spot request IDs (5 distinct) | 1 | 5 | 1 research console log |
   | Security-group ID | 1 | 2 | 1 research note |
   | IAM user name | 1 | 1 | 1 ADR |
   | Local OS user in paths / WSL `-u` | 4 | 6 | 3 docs, 1 research note |

   17 content files changed in total (several carry more than one type).
3. **Fail-closed guard.** [`scripts/check-public-ids.sh`](../../scripts/check-public-ids.sh) scans every git-tracked text file for identifier *shapes*: 12-digit account IDs next to an account/owner word, in an ARN or in an ECR host; AWS access key IDs; EC2/VPC resource IDs; Spot request IDs; Route 53 hosted-zone IDs (after a zone word, or bare `Z` + digit form). It holds **generic patterns only**, never a real value. Its allowlist holds public AWS-documentation example values only. On a hit it prints `file:line class` and **not** the matched text, because CI logs are public. `--self-test` plants synthetic tokens of every class in a throwaway repo (built at runtime) and fails if any class is missed or if an allowlisted example or a context-free number is flagged.
4. **Wired into `qemu-smoke.sh`** as its first step (self-test, then scan). That script already runs in both default `smoke.yml` jobs and in Docker smoke, so the guard is CI-gated without touching `.github/workflows/`. Marker lines: `public-ids: self-test ok classes=7`, `public-ids: ok files=N hits=0`.
5. **History is not rewritten.** The replaced values remain in earlier commits and in already-merged PR diffs. Rewriting history or force-pushing needs a separate sponsor decision after a risk evaluation. This ADR does not decide that.

## Evidence

- Guard on this branch: `public-ids: self-test ok classes=7`, `public-ids: ok files=… hits=0` (bash and dash).
- Guard on unmodified `main` (script copied into a scratch worktree): **FAIL hits=77** (raw matches). Distinct `file:line class` pairs: account-id-ctx 12, route53-zone-id 16, ec2-resource-id 26, spot-request-id 5. Every line that carried one of the values above was flagged, with no extra lines.
- CI: see the honesty ledger row for this PR.

## Limits (non-claims)

- A bare 12-digit number with **no** account/ARN/ECR context is not flagged. GitHub Actions job IDs have the same shape and appear throughout the ledger. The guard is a ratchet for the common shapes, not a DLP product.
- It does not scan git history, PR descriptions, issue comments or CI logs.
- Public, intended identifiers stay: the GitHub owner name in repo URLs, the public docs domain, git author metadata, public AWS region / instance-type names, GitHub run/job IDs, QEMU user-net addresses (10.0.2.x) and loopback.
- Not a “secure” or “compliant” claim.

## Consequences

- The open EL0 stack PRs (#139–#144) must pass this guard after they rebase on a `main` that contains it.
- New cloud work must write placeholders or env vars, never literal account or resource IDs.
