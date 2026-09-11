# Session memory — 2026-09-11 (docs Pages site)

Fetched `origin/main` `e80dc93` (#28 merged). Branch `cursor/docs-pages-site-c371`.

Picked mdBook 0.5.4 over MkDocs: Rust-kernel repo, single binary, `cname` in `book.toml`, source stays `docs/`. `SUMMARY.md` + landing `docs/index.md` + `docs/website.md` (local build, Pages, DNS checklist) + curated `docs/research.md` (vaults stay on GitHub; daily briefs not ingested into the book).

Workflow `.github/workflows/pages.yml`: PR = build + CNAME check; `main` = `upload-pages-artifact` + `deploy-pages`. Repo-root `CNAME` and generated `book/CNAME` both `ctos.artof.link`.

`gh api repos/artofdream/ctos` → `has_pages: false`, homepage null, Pages API 404. Documented the Settings click. Did not call write APIs.

Local probe: `./scripts/docs-build.sh` downloaded the official linux-gnu tarball, `mdbook build` ok.

Sponsor follow-up: DNS is Route 53, account `737290977112`, region `us-east-1`. Documented CLI + `scripts/route53-ctos-cname.json`. This VM: `dig` already CNAME to `artofdream.github.io.`; no AWS CLI/creds so the hosted-zone row is Unknown; HTTPS cert mismatch + HTTP 404 + `has_pages: false` so reachability stays Planned.

Do not self-merge. GitHub author of #30 is `artofdream`; merge hat `cursor[bot]`. MRC COMMENT landed on `443ff26`.
