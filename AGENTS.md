# AGENTS.md — ctos session protocol

This file is the source of truth for how agents work in this repo. Frozen product requirements live in [docs/02-requirements/fr-nfr.md](docs/02-requirements/fr-nfr.md) (FR-01–FR-15, NFR-01–NFR-14). Do not invent new IDs in chat; add them via a GitHub issue plus an ADR/docs change.

ctos is a **GitHub** project (`artofdream/ctos`). Use `gh` for issues, PRs, and reviews. Do not assume GitLab, `glab`, or a GitLab wiki. Learning-kernel docs are on GitHub Pages (mdBook) at `https://ctos.artof.link` — see [docs/website.md](docs/website.md). That URL is **Verified** (2026-09-11 after #30); do not invent a GitLab Pages SOP.

The practice here is a **ctos-native harness**: honesty (claim vs probe), fail-closed sensors, document-first, a four-vault second brain, and no self-merge. After M9 the same weight sits on **three pillars** — antifragility, security, performance ([ADR-011](docs/03-adr/ADR-011-three-pillars.md), [pillars.md](docs/framework/pillars.md)). It is *inspired by* harness-engineering / honesty practices ([architecture.artof.link](https://architecture.artof.link/) as prior art). ctos is its own OS-kernel project with its own vocabulary. Do not import florist platform, 14-hat maps, or AEA role names.

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
4. Do not self-approve or merge the PR you authored. See [ADR-002](docs/03-adr/ADR-002-pr-identity-split.md).

## Thin roles (`ctos-*` only)

| Role | Skill | Job |
| --- | --- | --- |
| Knowledge Guardian | `.cursor/skills/ctos-knowledge-guardian/` | Keep vision, ADRs, ledger, and second-brain vaults accurate |
| Coherence Guardian | `.cursor/skills/ctos-coherence-guardian/` | Docs, code, and claims stay consistent; no status inflation |
| Kernel Engineer | `.cursor/skills/ctos-kernel-engineer/` | `no_std` AArch64 kernel work (QEMU `virt`, UART) |
| MR Coordinator | `.cursor/skills/ctos-mr-coordinator/` | Independent review gate; **never** self-approve |

One human or agent may wear a builder hat in a session. The merge/approve hat is a different job.

## Solo era — author ≠ merger (two identities)

Stay solo as long as you need. **Do not** enable GitHub “authors can approve their own PRs.” Same login `APPROVE` is still self-review. Do **not** add a required-approval ruleset until a second human exists.

Quality is the **notation of roles** plus a **distinct merger**: who authored, who reviewed (MRC `COMMENT`), who merged. Decision: [ADR-002](docs/03-adr/ADR-002-pr-identity-split.md). Prior practice: Café Fausse `pr-coordinator` (identity split only — not the restaurant).

**Second reviewer:** a **new** Cursor session with `ctos-mr-coordinator` (not the authoring session). It writes `COMMENT` / `REQUEST_CHANGES` (ledger vs probes, frozen FR/NFR). It does not merge.

**Who merges** — `artofdream` (owner) vs `cursor[bot]` (Cursor GitHub App). Use the GitHub PR **`author` login** (`gh pr view --json author`), not `Co-authored-by` and not a brief that “expects” the bot.

| Who GitHub shows as author | Who writes the review | Who merges |
| --- | --- | --- |
| `artofdream` | New MRC session (`COMMENT`) | `cursor[bot]` after this-run green checks (when CI exists) and Bugbot resolved-or-declined. Owner must not Approve or merge. |
| `cursor[bot]` | Owner, optionally plus MRC `COMMENT` | **`artofdream`**. The bot must not merge. |

Cloud agents often open PRs as `artofdream` (commits may list `cursoragent`). That is still owner-opened → **`cursor[bot]` merges**. No self-merge.

**Historical exception (Track A A1–A9, 2026-09-11–2026-09-12):** #49, #52–#57, #59, #60 were `author=artofdream` and `mergedBy=artofdream`. Named in [ADR-002](docs/03-adr/ADR-002-pr-identity-split.md). Do not rewrite those PRs. Do not repeat the pattern.

If `cursor[bot]` `APPROVE` returns 403, do not block a valid bot merge of an owner-authored PR. Missing Approve is not a fail. Smoke CI exists (ledger); do not claim green checks that were not probed on **this** PR.

Copy-paste to start MRC:

```
You are ctos MR Coordinator only. Review https://github.com/artofdream/ctos/pull/<N>.
Read AGENTS.md, ADR-002, docs/framework/honesty-ledger.md, docs/02-requirements/fr-nfr.md.
Do not push code. Do not APPROVE if you authored. Do not merge.
Leave COMMENT or REQUEST_CHANGES. Name who authored / reviewed / should merge.
Check claims vs probes.
```

## Edit style

- Prefer small, focused diffs. One milestone per PR.
- Plain English. No florist or shop metaphors.
- Stay on the AArch64 QEMU `virt` / PL011 path ([ADR-003](docs/03-adr/ADR-003-primary-isa-aarch64.md)). Do not restore `bootloader` 0.9 or VGA as primary.
- Status words need a probe. File presence is not QEMU boot. File presence is not a “secure OS” or a bench.
- Do not mint FR-16+ or NFR-15+ in chat. Frozen set is [docs/02-requirements/fr-nfr.md](docs/02-requirements/fr-nfr.md). NFR-05 / NFR-07 / NFR-10 text was revised under [ADR-011](docs/03-adr/ADR-011-three-pillars.md).
- Optimize only with a probe. Do not claim “secure OS” or a product “the kernel is W^X.” Heap NX is [ADR-012](docs/03-adr/ADR-012-wx-nx-heap-stacks.md); guard holes are [ADR-014](docs/03-adr/ADR-014-linker-stack-guard-pages.md); identity-image RO+NX is [ADR-015](docs/03-adr/ADR-015-ro-nx-text-data.md). File presence is not that probe.

## Tracker

- Issues and PRs: GitHub via `gh`.
- Memory: `research/` in this git repo, not a separate wiki.
