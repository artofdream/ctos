# Daily brief — 2026-09-11 (docs plain English + mermaid)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/51 (`cursor/docs-plain-english-diagrams-e4a9`). Parent is `main` `f86785b` (#30 mdBook site already merged). Did **not** wait on open #29 / #49.

Live site `https://ctos.artof.link` is **Verified** (this VM): deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200 + Driving principles + Pages API `cname` / `https_enforced` / cert approved. Landing no longer says the custom domain is Planned.

`./scripts/docs-build.sh` **Verified** here: mdBook 0.5.4 + mdbook-mermaid 0.17.1; mermaid nodes on landing / overview / pillars. Generator only.

Did not create `docs/framework/principles.md` (that file lives on #29). Landing is the visitor-facing principles SoT. No `src/` changes.

## Do next

1. Pages PR build [34654325467](https://github.com/artofdream/ctos/actions/runs/34654325467) is green. Stay draft until MRC; author does not merge.
2. MRC `COMMENT` in a new session. Author does not merge (ADR-002).
3. #29 / #49 should rebase onto this after merge if they still touch ledger/README; do not drop A1/kernel from #49.

## Honesty

- Did not invent other Verified claims (Route 53 API LIST still Unknown; github.io trailing slash 404).
- No “secure OS,” “apps run,” “supports FAT,” or “EL0 isolated.”
