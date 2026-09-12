# Daily brief — 2026-09-12 (ADR-002 Track A merge-hat honesty)

## Where we stopped

Parent is `main` `ba6541c` (A9 / #60). Branch `cursor/adr-002-track-a-merge-hat-0e11`. Docs-only.

Named Track A A1–A9 same-login merge as a **historical exception** in [ADR-002](../../docs/03-adr/ADR-002-pr-identity-split.md). Restated going-forward: GitHub `author` is the identity that matters; owner-opened → `cursor[bot]` merges; bot-opened → `artofdream` merges. Cloud PRs opened as `artofdream` (commits often `cursoragent`) are still owner-opened.

`gh pr view --json author,mergedBy` on 2026-09-12: #49, #52–#57, #59, #60 were `artofdream` / `artofdream`. Not rewritten. Distinct-identity merge on this repo stays **Unknown**.

Cloud `./scripts/docs-build.sh` **Verified** (`mdbook v0.5.4`, mermaid 0.17.1, Linux auto-install, `docs-build: ok`). No `src/` edit. Kernel `qemu-smoke` not run. GitHub author of this PR is `artofdream`; merge hat is `cursor[bot]`.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). This cloud PR is expected `artofdream`-authored; merge hat is `cursor[bot]`.
2. After this PR lands, the next merge must show `author` ≠ `mergedBy` before anyone claims the split is practiced.

## Honesty

- File presence of the amendment is not a `cursor[bot]` merge probe.
- Did not rewrite Track A PR pages. Did not mint FR/NFR IDs.
