# Docs website + DNS

The published site is this mdBook (`book.toml`, **`src = "docs"`**). The website and the repo are the **same markdown**. Do not keep a second copy of the overview pages.

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

Frozen IDs touched by the publish path: [FR-14](02-requirements/fr-nfr.md) (document-first path stays ahead of code), [NFR-06](02-requirements/fr-nfr.md) (honesty), [NFR-13](02-requirements/fr-nfr.md) (written acceptance matches what is proven). No new FR/NFR IDs.

## Local build

You need [mdBook](https://github.com/rust-lang/mdBook) **0.5.4** (pinned in `.github/workflows/pages.yml` and `scripts/docs-build.sh`).

```bash
./scripts/docs-build.sh          # downloads mdBook if missing, then `mdbook build`
# or, if mdbook is already on PATH:
mdbook build                     # writes ./book/
mdbook serve                     # http://localhost:3000
```

`create-missing` is off: a `SUMMARY.md` link to a missing file fails the build. Output directory `book/` is gitignored.

A successful local `mdbook build` is a **generator** probe. It is not “the website is published.”

## Production URLs

| URL | Role | Status word |
| --- | --- | --- |
| `https://ctos.artof.link` | Intended custom domain | **DNS in place.** HTTPS serving the book is **Planned** until Pages is enabled, the repo custom domain is set, and a fetch succeeds |
| `https://artofdream.github.io/ctos/` | GitHub Pages project-site fallback | **Unknown** until a `pages` workflow on `main` is green |

Do not write Verified on either row from file presence alone.

## GitHub Pages workflow

`.github/workflows/pages.yml`:

- **Pull requests:** `mdbook build` + check that the output `CNAME` is `ctos.artof.link`. Does **not** deploy.
- **Push to `main`:** build, upload `actions/upload-pages-artifact`, deploy with `actions/deploy-pages`.

### One-time Settings (sponsor / repo admin)

The GitHub API on this repo currently reports `has_pages: false`. A workflow file does not flip that by itself. Someone with admin on `artofdream/ctos` must:

1. **Settings → Pages → Build and deployment → Source:** GitHub Actions.
2. After the first successful `pages` deploy on `main`, **Settings → Pages → Custom domain:** `ctos.artof.link`.
3. Wait for GitHub’s DNS check, then enable **Enforce HTTPS** (often delayed until the certificate exists).
4. Optional: set the repo **Homepage** to `https://ctos.artof.link` only after HTTPS serves the book. Leave it null until then.

GitHub’s `github-pages` environment is created on the first `actions/deploy-pages` run. Prefer a protection rule so only `main` deploys to it.

A `CNAME` file in the artifact (from `book.toml` `cname = "ctos.artof.link"`, also the repo-root `CNAME`) does **not** by itself register the custom domain. Settings (or the Pages API) still has to list the hostname. See [GitHub: publishing with Actions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site#publishing-with-a-custom-github-actions-workflow).

## DNS — Amazon Route 53 (CNAME in place; HTTPS still Planned)

`artof.link` is hosted on **Amazon Route 53**. The sponsor states the `ctos` CNAME is **already created**. Do **not** run `CREATE` again.

| Field | Value |
| --- | --- |
| AWS account | `737290977112` |
| Region for CLI | `us-east-1` (Route 53 API is global) |
| Hosted zone name | `artof.link` |
| Hosted zone ID | `Z1178AFMV41RWP` (sponsor-stated) |
| Record | `CNAME` `ctos.artof.link.` → `artofdream.github.io.` (trailing dot) |

This is a **project site** (`artofdream.github.io/ctos/`) on a **subdomain**. Do **not** CNAME to `artofdream.github.io/ctos`. Do not touch the `artof.link` apex.

Public `dig CNAME ctos.artof.link` on 2026-09-11 (this VM, twice) returned `artofdream.github.io.` **DNS is in place** at the resolver. That is not a Route 53 API `list-resource-record-sets` from this environment (no AWS CLI / credentials here). Next session may **LIST** zone `Z1178AFMV41RWP` to confirm; skip `change-resource-record-sets` unless the row is missing.

```bash
export AWS_REGION=us-east-1
aws sts get-caller-identity --query Account --output text   # expect 737290977112
aws route53 list-resource-record-sets --hosted-zone-id Z1178AFMV41RWP \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"
```

`scripts/route53-ctos-cname.json` is a leftover `CREATE` batch for recovery only.

### Still needed: GitHub Pages custom domain + HTTPS

A CNAME to `artofdream.github.io` is **not** “the docs are live.” `has_pages` was still false on 2026-09-11. After merge:

1. **Settings → Pages → Source:** GitHub Actions (one-time).
2. Green `pages` deploy on `main`.
3. **Settings → Pages → Custom domain:** `ctos.artof.link`, wait for GitHub’s DNS check, then **Enforce HTTPS**.
4. Optional: repo Homepage → `https://ctos.artof.link` only after HTTPS serves the book.

Until those steps, `https://ctos.artof.link` may TLS-mismatch or 404. Reachability stays **Planned**. Do not claim the URL serves docs.

### Probes

```bash
dig CNAME ctos.artof.link +short    # expect artofdream.github.io.  (in place)
curl -sSI https://ctos.artof.link   # 200 + cert for this name only after Pages binds it
```

## Honesty

| Claim | Probe | Until then |
| --- | --- | --- |
| mdBook builds this tree | `./scripts/docs-build.sh` (or `mdbook build`) exit 0 | — |
| Pages workflow exists | Read `.github/workflows/pages.yml` | File presence only |
| Pages workflow builds a PR | Green `pages` run on this branch (build job; deploy skipped) | See honesty ledger |
| Docs website published | Green `pages` workflow on `main` **and** a fetch of the github.io or custom URL | **Unknown** |
| Public CNAME `ctos.artof.link` | `dig CNAME ctos.artof.link +short` | **Verified** — DNS in place (`artofdream.github.io.`) |
| Route 53 API row in `737290977112` / `Z1178AFMV41RWP` | `list-resource-record-sets` as that account | **Unknown** here (no AWS CLI). Sponsor states CREATE already done. Do not CREATE again. |
| Custom domain reachability | Pages lists the hostname **and** HTTPS 200 with a matching cert | **Planned** (`has_pages: false`; do not claim the URL serves docs) |

Do not say “secure OS,” “EL0 isolated,” or that QEMU boot was proven by this docs PR.
