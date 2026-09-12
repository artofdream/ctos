# Docs website + DNS

The published site is this mdBook (`book.toml`, **`src = "docs"`**). The website and the repo are the **same markdown**. Visitor-facing source of truth is the [landing](index.md) plus `docs/overview/*`. `docs/framework/*` is for deep links (ledger, pillars, threat model). Do not keep a second marketing copy.

Required site chapters (sidebar + landing). A missing file fails `mdbook build` (`create-missing = false`) and `scripts/docs-build.sh`:

| Website page | Source file |
| --- | --- |
| What can run today | `docs/overview/what-can-run.md` |
| Building or porting | `docs/overview/porting.md` |
| Filesystem (memfs + FAT16) | `docs/overview/filesystem.md` |
| Hosting apps / containers | `docs/overview/hosting-apps.md` (guest OCI/Docker: **non-goal**, [ADR-029](03-adr/ADR-029-containers-nongoal.md)) |
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

Auto-install keys on `uname -s`:`uname -m`. Linux stays `x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-musl`. macOS uses the published `x86_64-apple-darwin` / `aarch64-apple-darwin` tarballs for mdBook and mdbook-mermaid. Other hosts: install both pins yourself. `mdbook --version` prints `mdbook v0.5.4`; the script strips the leading `v` before comparing to pin `0.5.4`.

`create-missing` is off: a `SUMMARY.md` link to a missing file fails the build. Output directory `book/` is gitignored.

A successful local `mdbook build` is a **generator** probe. It is not a new “the website is published” claim. The live URL is a separate ledger row.

## Mobile / narrow viewport

Sponsor bar (same as `architecture.artof.link`): viewport, readable type, usable nav on a phone, no page-wide horizontal overflow. Tables, `pre`, and Mermaid may scroll *inside* their widget.

**Stock mdBook 0.5.4 already has** (audited on the live HTML 2026-09-12):

- `<meta name="viewport" content="width=device-width, initial-scale=1">`
- Sidebar toggle (`#mdbook-sidebar-toggle`) plus `--sidebar-width: min(300px, 80vw)`
- `#mdbook-menu-bar { flex-wrap: wrap }`
- `.table-wrapper { overflow-x: auto }` around markdown tables
- `.content img { max-width: 100% }`

**Gaps closed in `docs/theme.css` + mermaid extras** (this tree):

- `pre` / code: `overflow-x: auto` so long lines do not widen the page
- Table cells: `overflow-wrap`; wide tables keep a `min-width` and scroll
- Touch: 44px minimum on sidebar links, menu icons, and mobile chapter arrows (≤768px)
- Body type stays 16px; heading strings may wrap
- Mermaid: on ≤768px, `useMaxWidth` is off so diagrams stay readable and the `.mermaid` box scrolls (`docs/mermaid.css`, `docs/mermaid-init.js`)

Desktop content column stays `--content-max-width: 750px`. Do not treat this section as a live-site deploy probe.

### Narrow-viewport probe

After `./scripts/docs-build.sh`, `./scripts/docs-mobile-probe.sh` serves `book/` and uses headless Chrome. An iframe forces the chapter width so `--dump-dom` cannot fake a wider layout. It records whether `documentElement.scrollWidth` exceeds `clientWidth` on the landing, the honesty ledger, and this page.

2026-09-12 this cloud VM (`Google Chrome 148.0.7778.96`): landing overflow=no at **320 / 768 / 1280** (`client` = `scroll`); ledger and this page overflow=no at 320. Internal `.table-wrapper` / `pre` / `.mermaid` scroll is allowed. The numeric probe maps `cdn.jsdelivr.net` to `127.0.0.1` so mermaid.js cannot hang Chrome (diagrams stay as source text for that run). A green local probe is not “https://ctos.artof.link is mobile-verified” until a post-merge Pages fetch repeats it.

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
| Narrow viewport / mobile layout | `./scripts/docs-mobile-probe.sh` after a local build (320 / 768 / 1280; landing + ledger + this page) | **Verified** locally 2026-09-12 (iframe overflow=no). Not a live-site mobile claim until a post-merge Pages fetch. |
| Pages workflow exists | Read `.github/workflows/pages.yml` | File presence only |
| Pages workflow builds a PR | Green `pages` run on this branch (build job; deploy skipped) | See honesty ledger |
| Docs website published | Green `pages` workflow on `main` **and** HTTPS fetch of `https://ctos.artof.link` | **Verified** — deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200 + Driving principles |
| Public CNAME `ctos.artof.link` | `dig CNAME ctos.artof.link +short` | **Verified** — DNS in place (`artofdream.github.io.`) |
| Route 53 API row in `737290977112` / `Z1178AFMV41RWP` | `list-resource-record-sets` as that account | **Unknown** here (no AWS CLI). Sponsor states CREATE already done. Do not CREATE again. |
| Custom domain reachability | Pages lists the hostname **and** HTTPS 200 with a matching cert | **Verified** — Pages API `cname` + `https_enforced` + cert approved; `curl -sSI https://ctos.artof.link` HTTP 200 |

Do not say “secure OS,” “EL0 isolated,” or that QEMU boot was proven by this docs PR.
