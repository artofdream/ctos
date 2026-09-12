# Daily brief — 2026-09-12 (docs-build Darwin triples + version pin)

## Where we stopped

Rebase of conflicting draft [#50](https://github.com/artofdream/ctos/pull/50) onto current `main` (`ae7d2b8` / A7). Branch `cursor/docs-build-darwin-pin-4116`.

`scripts/docs-build.sh` now keys auto-install on `uname -s`:`uname -m` for **mdBook and mdbook-mermaid** (Linux gnu/musl + Darwin apple triples) and strips clap’s leading `v` before comparing to pin `0.5.4`. Mermaid preprocessor from #51 is kept. No kernel change.

## Do next

1. MRC `COMMENT` on the follow-up PR. If this session updates #50: author `artofdream` → merge `cursor[bot]`. If a replacement PR is `cursor[bot]`-authored: merge hat is `artofdream`.
2. Darwin auto-install remains **Unknown** until someone runs the script on macOS with no PATH binaries.
3. Do not self-merge (ADR-002). Keep draft until this-SHA mdBook CI is grepped green.

## Honesty

- Release-asset names existing is not a Darwin download probe.
- Pages / `https://ctos.artof.link` stays the existing Verified row. This PR does not republish that claim as Unknown.
