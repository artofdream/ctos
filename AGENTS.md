# AGENTS.md — ctos session protocol

This file is the source of truth for how agents work in this repo.

ctos is a **GitHub** project (`artofdream/ctos`). Use `gh` for issues, PRs, and reviews. Do not assume GitLab, `glab`, a GitLab wiki, or a Pages publish SOP.

The practice here is a **ctos-native harness**: honesty (claim vs probe), fail-closed sensors, document-first, a four-vault second brain, and no self-merge. It is *inspired by* harness-engineering / honesty practices ([architecture.artof.link](https://architecture.artof.link/) as prior art). ctos is its own OS-kernel project with its own vocabulary. Do not import florist platform, 14-hat maps, or AEA role names.

## Session start

1. Read the latest file in `research/daily-briefs/` (if any).
2. Read the newest notes in `research/random-thoughts/`.
3. Read [docs/framework/honesty-ledger.md](docs/framework/honesty-ledger.md). Treat every status word as a claim.
4. Read this file and the skill that matches the work (see roles below).
5. Confirm the loop unit: **one milestone → one branch → one PR**.

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

## Edit style

- Prefer small, focused diffs. One milestone per PR.
- Plain English. No florist or shop metaphors.
- Stay on `bootloader` 0.9 / `volatile` 0.2 / `spin` 0.5. Do not migrate to bootloader 0.10 in a drive-by.
- Status words need a probe. File presence is not QEMU boot.

## Tracker

- Issues and PRs: GitHub via `gh`.
- Memory: `research/` in this git repo, not a separate wiki.
