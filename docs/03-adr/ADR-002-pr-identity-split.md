# ADR-002 — PR author ≠ merger (two GitHub identities)

- Status: Accepted
- Date: 2026-09-08
- Amended: 2026-09-12 (Track A historical exception + going-forward restatement)

## Context

NFR-12 says implementer ≠ merge approver. On a solo GitHub login, a second Cursor agent cannot `APPROVE` a PR that login opened (GitHub self-APPROVE rule). Enabling “authors can approve their own PRs” would count same-login review as a gate. That is theater.

Café Fausse (`artofdream/aea-interactive-design`, `.cursor/skills/pr-coordinator/SKILL.md`) already probed this: **`cursor[bot]`** is the distinct second identity. Same principle applies here. This ADR does not import that repo’s restaurant/SRS content.

Cloud / Cursor agents on this repo often **open** PRs as **`artofdream`**. Commits commonly list `cursoragent` plus `Co-authored-by: artofdream`. That is still an owner-opened PR. A trailer does not change the GitHub `author` login and does not authorize a same-login merge.

## Decision

Reuse that split on ctos:

1. Do **not** enable GitHub author self-APPROVE. Do **not** add a required-approval ruleset until a second human exists.
2. **Author does not merge** their own PR.
3. Identities: **`artofdream`** (owner) and **`cursor[bot]`** (Cursor GitHub App, already used for cloud PRs). Do not install another App or mint a second personal account for this.
4. Written MRC (`ctos-mr-coordinator`) is the review-in-the-room. Prefer `COMMENT` / `REQUEST_CHANGES`. Record **who authored / who reviewed / who merges** on the PR. Name those logins from `gh pr view --json author,mergedBy` (or the GitHub PR header), not from a daily-brief guess.
5. If `cursor[bot]` REST `APPROVE` returns 403 (probed on Café Fausse #27; **unprobed on ctos**), do not block a valid **`cursor[bot]` merge** of an `artofdream`-authored PR after this-run green checks (when CI exists) and Bugbot resolved-or-declined. Missing Approve is not a reason to treat that merge as invalid.
6. Owner PAT / `artofdream` still must **not** `APPROVE` or merge an `artofdream`-authored PR.

### Going-forward (2026-09-12 restatement)

The **GitHub PR `author` login** is the identity that matters.

| Who GitHub shows as author | Who writes the review | Who merges |
| --- | --- | --- |
| `artofdream` | New MRC session (`COMMENT`) | **`cursor[bot]`** after this-run green checks (when CI exists) and Bugbot resolved-or-declined. Owner must **not** Approve or merge. |
| `cursor[bot]` (cloud / this agent when GitHub actually attributes the PR to the App) | Owner, optionally plus MRC `COMMENT` | **`artofdream`**. The bot must **not** merge. |

No self-merge. Owner-opened → bot merges. Bot-opened → owner merges.

### Historical exception — Track A (A1–A9)

**Named exception. Do not rewrite those PR pages.** Sponsor directed this cleanup after Track A closed its first cuts.

- **When:** 2026-09-11–2026-09-12
- **Scope:** Track A first-cut PRs A1–A9: [#49](https://github.com/artofdream/ctos/pull/49) (A1), [#52](https://github.com/artofdream/ctos/pull/52) (A2), [#53](https://github.com/artofdream/ctos/pull/53) (A3), [#54](https://github.com/artofdream/ctos/pull/54) (A4), [#55](https://github.com/artofdream/ctos/pull/55) (A5), [#56](https://github.com/artofdream/ctos/pull/56) (A6), [#57](https://github.com/artofdream/ctos/pull/57) (A7), [#59](https://github.com/artofdream/ctos/pull/59) (A8), [#60](https://github.com/artofdream/ctos/pull/60) (A9)
- **What `gh` showed (2026-09-12):** each of those PRs had **`author=artofdream` and `mergedBy=artofdream`**. Commits were typically `cursoragent` with `Co-authored-by: artofdream`. That is same-login merge. It does **not** satisfy this ADR.
- **What this is not:** not a claim those kernel or docs miles are invalid (their probes stay on their own ledger rows); not a rewrite of git history; not permission to repeat the pattern.
- **Earlier miles:** many pre-Track-A merged PRs show the same GitHub `author`/`mergedBy` pair. This note names **Track A** as the sponsor-directed exception. It does not relitigate those older PR pages.

After this amendment, a same-login merge is a process miss, not an undocumented habit.

## Consequences

- Probe the GitHub `author` field before naming the merge hat. Chat “expected `cursor[bot]`” is not a probe.
- [#2](https://github.com/artofdream/ctos/pull/2): `gh` on 2026-09-12 shows `author=artofdream`, `mergedBy=artofdream`. The 2026-09-08 sentence that called it `cursor[bot]`-authored is withdrawn.
- Smoke CI exists and has been probed on this repo (honesty ledger CI rows). “Green checks” on a **new** PR still need that PR’s run. Missing this-run green is not green.
- `cursor[bot]` **merge** and App `APPROVE` on **this** repo stay **Unknown** until a distinct-identity merge is probed here. Do not copy Café Fausse #16/#27 as a ctos probe. Track A same-login merges are not that probe.
