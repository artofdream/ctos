# AGENTS.md — ctos session protocol

This file is the source of truth for how agents work in this repo. Frozen product requirements live in [docs/02-requirements/fr-nfr.md](docs/02-requirements/fr-nfr.md) (FR-01–FR-15, NFR-01–NFR-14). Do not invent new IDs in chat; add them via a GitHub issue plus an ADR/docs change.

ctos is a **GitHub** project (`artofdream/ctos`). Use `gh` for issues, PRs, and reviews. Do not assume GitLab, `glab`, a GitLab wiki, or a Pages publish SOP.

The practice here is a **ctos-native harness**: honesty (claim vs probe), fail-closed sensors, document-first, a four-vault second brain, and no self-merge. It is *inspired by* harness-engineering / honesty practices ([architecture.artof.link](https://architecture.artof.link/) as prior art). ctos is its own OS-kernel project with its own vocabulary. Do not import florist platform, 14-hat maps, or AEA role names.

## Session start

1. Read the latest file in `research/daily-briefs/` (if any).
2. Read the newest notes in `research/random-thoughts/`.
3. Read [docs/framework/honesty-ledger.md](docs/framework/honesty-ledger.md). Treat every status word as a claim.
4. Read [docs/02-requirements/fr-nfr.md](docs/02-requirements/fr-nfr.md) when the work touches behavior; cite frozen FR/NFR IDs in the PR.
5. Read this file and the skill that matches the work (see roles below).
6. Confirm the loop unit: **one milestone → one branch → one PR**.

## Session end

1. Update the honesty ledger if you probed something (or leave it Unknown).
2. Write a short handoff in `research/daily-briefs/` (date in the filename).
3. Drop raw session memory in `research/random-thoughts/` — do not stuff the PR with chat logs.
4. Do not self-approve the PR you authored.

## Thin roles (`ctos-*` only)

| Role | Skill | Job |
| --- | --- | --- |
| Knowledge Guardian | `.cursor/skills/ctos-knowledge-guardian/` | Keep vision, ADRs, ledger, and second-brain vaults accurate |
| Coherence Guardian | `.cursor/skills/ctos-coherence-guardian/` | Docs, code, and claims stay consistent; no status inflation |
| Kernel Engineer | `.cursor/skills/ctos-kernel-engineer/` | `no_std` kernel work on the tutorial-era stack |
| MR Coordinator | `.cursor/skills/ctos-mr-coordinator/` | Independent review gate; **never** self-approve |

One human or agent may wear a builder hat in a session. The merge/approve hat is a different job.

## Solo era (sponsor is the main developer)

Stay solo as long as you need. Do **not** turn on GitHub “authors can approve their own PRs,” and do **not** add a required-approval ruleset yet — with one GitHub account you would only merge via admin bypass, which is theater.

**Second reviewer:** start a **new** Cursor chat or cloud agent (not the session that wrote the PR). Point it at the PR URL and the `ctos-mr-coordinator` skill. That agent writes the review (ledger vs probes, scope, no invented FR/NFR IDs).

**Who clicks Merge:** you. A second agent on the same GitHub login **cannot** submit an Approve on a PR that login opened. Treat its review comments as the second pair of eyes; you are still the merge button.

| Who wrote the PR | Who reviews | Who merges |
| --- | --- | --- |
| You | New MRC agent (or you, slowly, against the ledger) | You, after the written review |
| Builder agent (this GitHub user) | You, optionally plus a new MRC agent | You |

Copy-paste to start MRC:

```
You are ctos MR Coordinator only. Review https://github.com/artofdream/ctos/pull/<N>.
Read AGENTS.md, docs/framework/honesty-ledger.md, docs/02-requirements/fr-nfr.md.
Do not push code, do not approve if you authored the commits, do not merge.
Leave a GitHub review (comment or request changes). Check claims vs probes.
```

Later, when a second GitHub identity exists (teammate, bot, or Copilot review), you can add a real required-approval ruleset. Until then, written MRC + human merge satisfies NFR-12 without locking `main`.

## Edit style

- Prefer small, focused diffs. One milestone per PR.
- Plain English. No florist or shop metaphors.
- Stay on `bootloader` 0.9 / `volatile` 0.2 / `spin` 0.5. Do not migrate to bootloader 0.10 in a drive-by.
- Status words need a probe. File presence is not QEMU boot.
- Do not mint FR-16+ or NFR-15+ in chat. Frozen set is [docs/02-requirements/fr-nfr.md](docs/02-requirements/fr-nfr.md).

## Tracker

- Issues and PRs: GitHub via `gh`.
- Memory: `research/` in this git repo, not a separate wiki.
