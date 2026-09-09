# ADR-002 — PR author ≠ merger (two GitHub identities)

- Status: Accepted
- Date: 2026-09-08

## Context

NFR-12 says implementer ≠ merge approver. On a solo GitHub login, a second Cursor agent cannot `APPROVE` a PR that login opened (GitHub self-APPROVE rule). Enabling “authors can approve their own PRs” would count same-login review as a gate. That is theater.

Café Fausse (`artofdream/aea-interactive-design`, `.cursor/skills/pr-coordinator/SKILL.md`) already probed this: **`cursor[bot]`** is the distinct second identity. Same principle applies here. This ADR does not import that repo’s restaurant/SRS content.

## Decision

Reuse that split on ctos:

1. Do **not** enable GitHub author self-APPROVE. Do **not** add a required-approval ruleset until a second human exists.
2. **Author does not merge** their own PR.
3. Identities: **`artofdream`** (owner) and **`cursor[bot]`** (Cursor GitHub App, already used for cloud PRs). Do not install another App or mint a second personal account for this.
4. Written MRC (`ctos-mr-coordinator`) is the review-in-the-room. Prefer `COMMENT` / `REQUEST_CHANGES`. Record **who authored / who reviewed / who merges** on the PR.
5. If `cursor[bot]` REST `APPROVE` returns 403 (probed on Café Fausse #27; **unprobed on ctos**), do not block a valid **`cursor[bot]` merge** of an `artofdream`-authored PR after this-run green checks (when CI exists) and Bugbot resolved-or-declined. Missing Approve is not a reason to treat that merge as invalid.
6. Owner PAT / `artofdream` still must **not** `APPROVE` or merge an `artofdream`-authored PR.

| Who opened the PR | Who writes the review | Who merges |
| --- | --- | --- |
| `artofdream` | New MRC session (`COMMENT`) | `cursor[bot]` after green + Bugbot terminal |
| `cursor[bot]` (cloud / this agent) | Owner, optionally plus MRC `COMMENT` | **`artofdream`** |

## Consequences

- [#2](https://github.com/artofdream/ctos/pull/2) is `cursor[bot]`-authored scaffold: owner merges after the MRC write-up.
- CI is still Planned on ctos. “Green checks” cannot be claimed until a workflow is probed. Fail closed: no CI yet is not “green.”
- `cursor[bot]` merge and App `APPROVE` on **this** repo stay Unknown until probed here. Do not copy Café Fausse #16/#27 as a ctos probe.
