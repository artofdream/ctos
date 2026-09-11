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

## DNS — sponsor action (Planned)

Do **not** claim DNS is configured until someone at the `artof.link` DNS host adds the record and a resolver answers.

This is a **project site** (`artofdream.github.io/ctos/`) using a **subdomain**. Prefer a CNAME. Do not point `artof.link` apex at this repo.

| Host / name | Type | Value | Notes |
| --- | --- | --- | --- |
| `ctos` (fqdn `ctos.artof.link`) | `CNAME` | `artofdream.github.io` | **Preferred.** No trailing path. Do **not** CNAME to `artofdream.github.io/ctos`. |

After the record exists, probe with something like `dig CNAME ctos.artof.link +short` (expect `artofdream.github.io.`) and then `curl -I https://ctos.artof.link`. Until those answer, the custom-domain row stays **Planned**.

Apex (`artof.link` itself) would need the A / ALIAS records GitHub documents for user/org sites. That is **out of scope** here.

If the DNS host already has an A record for `ctos`, remove it before adding the CNAME (CNAME cannot coexist with other data on the same name).

## Honesty

| Claim | Probe | Until then |
| --- | --- | --- |
| mdBook builds this tree | `./scripts/docs-build.sh` (or `mdbook build`) exit 0 | — |
| Pages workflow exists | Read `.github/workflows/pages.yml` | File presence only |
| Pages workflow builds a PR | Green `pages` run on this branch (build job; deploy skipped) | See honesty ledger |
| Docs website published | Green `pages` workflow on `main` **and** a fetch of the github.io or custom URL | **Unknown** |
| Custom domain `ctos.artof.link` | DNS CNAME answers + HTTPS fetch | **Planned** (sponsor DNS + Settings) |

Do not say “secure OS,” “EL0 isolated,” or that QEMU boot was proven by this docs PR.
