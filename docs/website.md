# Docs website + DNS

The published site is this mdBook (`book.toml`, source `docs/`). It is a documentation surface for the learning kernel, not a product site.

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
| `https://ctos.artof.link` | Intended custom domain (same `artof.link` family as other sponsor sites) | **Planned** until DNS responds and GitHub issues a cert |
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
4. Optional: set the repo **Homepage** to `https://ctos.artof.link` once DNS answers. Leave it null until then.

GitHub’s `github-pages` environment is created on the first `actions/deploy-pages` run. Prefer a protection rule so only `main` deploys to it.

A `CNAME` file in the artifact (from `book.toml` `cname = "ctos.artof.link"`, also the repo-root `CNAME`) does **not** by itself register the custom domain. Settings (or the Pages API) still has to list the hostname. See [GitHub: publishing with Actions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site#publishing-with-a-custom-github-actions-workflow).

## DNS — Amazon Route 53 (Planned until Pages shows the domain)

`artof.link` is hosted on **Amazon Route 53**.

| Field | Value |
| --- | --- |
| AWS account | `737290977112` |
| Region for the next-session CLI | `us-east-1` (hosted zone for `artof.link` is assumed there; the Route 53 API itself is global) |
| Hosted zone | `artof.link` (look up `HostedZoneId` — do not guess it) |
| Record name | `ctos` or `ctos.artof.link` (fqdn `ctos.artof.link.`) |
| Type | `CNAME` |
| Value | `artofdream.github.io.` (**trailing dot** — Route 53 expects a FQDN) |

This is a **project site** (`artofdream.github.io/ctos/`) on a **subdomain**. Do **not** CNAME to `artofdream.github.io/ctos`. Do **not** point the `artof.link` apex at this repo. Do **not** use a Route 53 alias to GitHub (GitHub documents a plain CNAME for subdomains).

Apex A / ALIAS records GitHub documents are for user/org sites — **out of scope**.

If `ctos` already has an A / AAAA / other CNAME, delete that set first. A CNAME cannot coexist with other data on the same name.

Copy-paste CLI for the **next** session (this session did **not** run it — no AWS CLI and no credentials here). Confirm the caller is account `737290977112` before writing.

```bash
export AWS_REGION=us-east-1
aws sts get-caller-identity --query Account --output text   # expect 737290977112

ZONE_ID=$(aws route53 list-hosted-zones-by-name --dns-name artof.link. \
  --query "HostedZones[?Name=='artof.link.'].Id" --output text)
ZONE_ID="${ZONE_ID##*/}"

# LIST FIRST. CREATE fails if the name already exists.
aws route53 list-resource-record-sets --hosted-zone-id "$ZONE_ID" \
  --query "ResourceRecordSets[?Name=='ctos.artof.link.']"

aws route53 change-resource-record-sets --hosted-zone-id "$ZONE_ID" \
  --change-batch file://scripts/route53-ctos-cname.json
```

`scripts/route53-ctos-cname.json` is the `CREATE` batch (`TTL` 300, value `artofdream.github.io.`). Use `UPSERT` only if you intend to overwrite an existing `ctos` CNAME.

Console path (same account): Route 53 → Hosted zones → `artof.link` → Create record → CNAME → Record name `ctos` → Value `artofdream.github.io`.

### After the record exists — GitHub Pages custom domain + HTTPS

Public DNS answering is **not** “the site is live.” `has_pages` was still false on 2026-09-11. After Settings → Pages → Source = GitHub Actions and a green `main` deploy:

1. **Settings → Pages → Custom domain:** `ctos.artof.link` (or Pages API `cname`). GitHub checks the CNAME.
2. Wait until the Pages UI shows the domain and DNS as checked.
3. Enable **Enforce HTTPS**. The certificate often lags the CNAME by minutes to an hour. A TLS name-mismatch or HTTP 404 means Pages has not bound the hostname yet — keep reachability **Planned**.
4. Optional: repo Homepage → `https://ctos.artof.link` only after HTTPS serves the book.

### Probes (do not skip)

```bash
dig CNAME ctos.artof.link +short    # expect artofdream.github.io.
curl -sSI https://ctos.artof.link   # expect HTTP 200 and a cert for this name
```

2026-09-11 this cloud VM: `dig` already returned `artofdream.github.io.` That is a **public resolver** probe, not a Route 53 API read of account `737290977112`. `curl -sSI https://ctos.artof.link` failed TLS (`no alternative certificate subject name matches`). HTTP to the name was GitHub `404`. Custom-domain reachability stays **Planned**.

## Honesty

| Claim | Probe | Until then |
| --- | --- | --- |
| mdBook builds this tree | `./scripts/docs-build.sh` (or `mdbook build`) exit 0 | — |
| Pages workflow exists | Read `.github/workflows/pages.yml` | File presence only |
| Pages workflow builds a PR | Green `pages` run on this branch (build job; deploy skipped) | See honesty ledger |
| Docs website published | Green `pages` workflow on `main` **and** a fetch of the github.io or custom URL | **Unknown** |
| Public CNAME `ctos.artof.link` | `dig CNAME ctos.artof.link +short` | **Verified** on 2026-09-11 this VM (`artofdream.github.io.`) — not a Route 53 API read |
| Route 53 row in account `737290977112` | `aws route53 list-resource-record-sets` as that account | **Unknown** (no AWS CLI / credentials in this environment) |
| Custom domain reachability | Pages UI lists the hostname **and** `curl -sSI https://ctos.artof.link` is 200 with a matching cert | **Planned** (2026-09-11: TLS name-mismatch; HTTP 404; `has_pages: false`) |

Do not say “secure OS,” “EL0 isolated,” or that QEMU boot was proven by this docs PR.
