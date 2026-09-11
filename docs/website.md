# Docs website + DNS

The published site is this mdBook (`book.toml`, **`src = "docs"`**). The website and the repo are the **same markdown**. Visitor-facing source of truth is the [landing](index.md) plus `docs/overview/*`. `docs/framework/*` is for deep links (ledger, pillars, threat model). Do not keep a second marketing copy.

Required site chapters (sidebar + landing). A missing file fails `mdbook build` (`create-missing = false`) and `scripts/docs-build.sh`:

| Website page | Source file |
| --- | --- |
| What can run today | `docs/overview/what-can-run.md` |
| Building or porting | `docs/overview/porting.md` |
| Filesystem (Planned) | `docs/overview/filesystem.md` |
| Hosting apps / containers | `docs/overview/hosting-apps.md` |
| KPIs / how we measure | `docs/overview/measure.md` |
| Prerequisites | `docs/overview/prerequisites.md` |
| Advantages | `docs/overview/advantages.md` |
| Drawbacks / limits | `docs/overview/limits.md` |

Frozen IDs touched by the publish path: document-first ([FR-14](02-requirements/fr-nfr.md)), honesty ([NFR-06](02-requirements/fr-nfr.md)), written acceptance matches what is proven ([NFR-13](02-requirements/fr-nfr.md)). No new FR/NFR IDs.

Diagrams use ```mermaid``` fences. `mdbook-mermaid` 0.17.1 wraps them; `docs/mermaid-init.js` loads mermaid **11.6.0** from jsDelivr in the browser. A local `mdbook build` without `mdbook-mermaid` on PATH fails (`book.toml` lists the preprocessor). Use `./scripts/docs-build.sh`.

## Local build

You need [mdBook](https://github.com/rust-lang/mdBook) **0.5.4** and [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid) **0.17.1** (pinned in `.github/workflows/pages.yml` and `scripts/docs-build.sh`).

```bash
./scripts/docs-build.sh          # downloads both binaries if missing, then `mdbook build`
# or, if both are already on PATH:
mdbook build                     # writes ./book/
mdbook serve                     # http://localhost:3000
```

`create-missing` is off: a `SUMMARY.md` link to a missing file fails the build. Output directory `book/` is gitignored.

A successful local `mdbook build` is a **generator** probe. It is not a new “the website is published” claim. The live URL is a separate ledger row.

## Production URLs

| URL | Role | Status word |
| --- | --- | --- |
| `https://ctos.artof.link` | Custom domain | **Verified.** HTTPS 200, cert for this name, landing includes Driving principles. Main deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) after #30. Pages API: `cname=ctos.artof.link`, `https_enforced=true`, cert approved (expires 2026-12-10). |
| `https://artofdream.github.io/ctos` | Project-site path (no trailing slash) | **Redirect** (301) to the custom domain on the 2026-09-11 probe. A trailing-slash `.../ctos/` 404’d. Do not treat github.io as a second live tree. |

Do not write Verified on a *new* deploy from file presence alone. Re-fetch after the next `main` Pages run if you change the claim.

## GitHub Pages workflow

`.github/workflows/pages.yml`:

- **Pull requests:** `mdbook build` + check that the output `CNAME` is `ctos.artof.link`. Does **not** deploy.
- **Push to `main`:** build, upload `actions/upload-pages-artifact`, deploy with `actions/deploy-pages`.

Settings already in use after #30: Source = GitHub Actions, custom domain `ctos.artof.link`, Enforce HTTPS on. Do not flip Source away from Actions. A `CNAME` file in the artifact (from `book.toml` `cname = "ctos.artof.link"`, also the repo-root `CNAME`) should stay in lockstep.

See [GitHub: publishing with Actions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site#publishing-with-a-custom-github-actions-workflow).

## DNS — Amazon Route 53 (CNAME in place)

`artof.link` is hosted on **Amazon Route 53**. The sponsor states the `ctos` CNAME is **already created**. Do **not** run `CREATE` again.

| Field | Value |
| --- | --- |
| AWS account | `737290977112` |
| Region for CLI | `us-east-1` (Route 53 API is global) |
| Hosted zone name | `artof.link` |
| Hosted zone ID | `Z1178AFMV41RWP` (sponsor-stated) |
| Record | `CNAME` `ctos.artof.link.` → `artofdream.github.io.` (trailing dot) |

This is a **project site** on a **subdomain**. Do **not** CNAME to `artofdream.github.io/ctos`. Do not touch the `artof.link` apex.

Public `dig CNAME ctos.artof.link` returns `artofdream.github.io.` **DNS is in place** at the resolver. That is not a Route 53 API `list-resource-record-sets` from this environment (no AWS CLI / credentials here). Next session may **LIST** zone `Z1178AFMV41RWP` to confirm; skip `change-resource-record-sets` unless the row is missing.

```bash
export AWS_REGION=us-east-1
aws sts get-caller-identity --query Account --output text   # expect 737290977112
aws route53 list-resource-record-sets --hosted-zone-id Z1178AFMV41RWP \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"
```

`scripts/route53-ctos-cname.json` is a leftover `CREATE` batch for recovery only.

### Probes

```bash
dig CNAME ctos.artof.link +short    # expect artofdream.github.io.
curl -sSI https://ctos.artof.link   # expect HTTP 200 (Verified 2026-09-11)
```

## Honesty

| Claim | Probe | Until then |
| --- | --- | --- |
| mdBook builds this tree | `./scripts/docs-build.sh` (or `mdbook build` with mermaid preprocessor) exit 0 | — |
| Pages workflow exists | Read `.github/workflows/pages.yml` | File presence only |
| Pages workflow builds a PR | Green `pages` run on this branch (build job; deploy skipped) | See honesty ledger |
| Docs website published | Green `pages` workflow on `main` **and** HTTPS fetch of `https://ctos.artof.link` | **Verified** — deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200 + Driving principles |
| Public CNAME `ctos.artof.link` | `dig CNAME ctos.artof.link +short` | **Verified** — DNS in place (`artofdream.github.io.`) |
| Route 53 API row in `737290977112` / `Z1178AFMV41RWP` | `list-resource-record-sets` as that account | **Unknown** here (no AWS CLI). Sponsor states CREATE already done. Do not CREATE again. |
| Custom domain reachability | Pages lists the hostname **and** HTTPS 200 with a matching cert | **Verified** — Pages API `cname` + `https_enforced` + cert approved; `curl -sSI https://ctos.artof.link` HTTP 200 |

Do not say “secure OS,” “EL0 isolated,” or that QEMU boot was proven by this docs PR.
