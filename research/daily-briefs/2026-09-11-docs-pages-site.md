# Daily brief — 2026-09-11 (docs website / GitHub Pages)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/30 (`cursor/docs-pages-site-c371`). Parent is `main` `e80dc93` (Merge PR #28 / ADR-020). One PR: mdBook docs site + Pages workflow + `CNAME` for `ctos.artof.link`. No kernel changes. No new FR/NFR IDs.

Local `./scripts/docs-build.sh` **Verified** on this cloud VM (mdBook 0.5.4, `book/CNAME` = `ctos.artof.link`). PR `pages` run [34651704287](https://github.com/artofdream/ctos/actions/runs/34651704287) **Verified** (`mdBook build`; deploy skipped). Generator / PR-build only.

`has_pages` was **false** and homepage null when this work started. Custom domain stays **Planned** (sponsor DNS + Settings). “Docs website published” stays **Unknown** until a `pages` workflow on `main` is green and a URL fetch succeeds.

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of this PR is expected `cursor[bot]`; merge hat is `artofdream`.
2. Repo admin: Settings → Pages → Source = GitHub Actions. After first `main` deploy, set custom domain `ctos.artof.link`.
3. Sponsor: DNS `CNAME ctos` → `artofdream.github.io` at the `artof.link` host. Do not claim DNS is live until a resolver answers.

## Honesty

- Did not claim the site is published or that DNS is configured.
- Did not claim “secure OS” or “EL0 isolated.”
