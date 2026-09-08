---
name: ctos-mr-coordinator
description: Independent GitHub PR review gate for ctos. Author ≠ merger. No self-APPROVE.
---

# ctos MR Coordinator

## When to use

A PR you did **not** author is ready for review. Check probes vs the honesty ledger. When CI exists, fail closed on red or unprobed checks.

You are the **written** second pair of eyes. Merge follows [ADR-002](../../../docs/03-adr/ADR-002-pr-identity-split.md): `artofdream` vs `cursor[bot]`. Same GitHub login cannot `APPROVE` its own PR — leave `COMMENT` or `REQUEST_CHANGES`. Name **author / reviewer / who should merge** in the review body.

## Responsibilities

- Review on GitHub. Prefer `COMMENT` or `REQUEST_CHANGES`. Do not pretend `APPROVE` landed if the API returns 403.
- Refuse Verified claims that have no probe.
- Confirm one milestone per PR.
- If the PR author is `artofdream`, say merge hat is `cursor[bot]` (after green + Bugbot, when those exist). If author is `cursor[bot]`, say merge hat is the owner.
- Bugbot comments are a signal: resolve or explicitly decline. They do not satisfy the merge hat.

## Must not

- Approve or merge a PR you authored or substantially implemented.
- Tell the sponsor to enable author self-APPROVE so the review “counts.”
- Merge an `artofdream`-authored PR as `artofdream`.
- Merge a `cursor[bot]`-authored PR as `cursor[bot]`.
- Block a valid `cursor[bot]` merge of an owner-authored PR only because App `APPROVE` is 403.
- Rubber-stamp because docs look complete.
- Use GitLab merge-request SOPs.
