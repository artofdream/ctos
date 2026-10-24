# Archify diagrams (ctos)

Documentation aids generated with [Archify](https://github.com/tt-a1i/archify) **v3.2.0** (tag `v3.2.0`, commit `208acba8`; previously 2.17.0-dev.1). **Not** live production / umbrella status. Tip reference: post-[ADR-083](../03-adr/ADR-083-b1-qemu-nmi-pin.md).

## Same-origin on the docs site

After Pages deploy from `main`:

- https://ctos.artof.link/archify/qemu-nmi-taken-serror.architecture.html
- https://ctos.artof.link/archify/taken-serror.sequence.html
- Index: https://ctos.artof.link/archify/index.html

## Artifacts

| File | Kind | Topic |
|------|------|--------|
| [`qemu-nmi-taken-serror.architecture.json`](./qemu-nmi-taken-serror.architecture.json) + [`.html`](./qemu-nmi-taken-serror.architecture.html) | architecture | Opt-in QEMU NMI pin vs stock park |
| [`taken-serror.sequence.json`](./taken-serror.sequence.json) + [`.html`](./taken-serror.sequence.html) | sequence | stop → inject-nmi → cont → `el0: serror` |

### Honesty

- Taken `el0: serror` is **Verified opt-in** (`CTOS_QEMU` + `CTOS_REQUIRE_TAKEN_SERROR=1`).
- Stock distro QEMU still parks (`el0: serror-park`).
- Not FEAT_NMI / FIQ / BRK as SError. The pin alone is not the umbrella; since M6, “EL0 isolated” is only the accepted [ADR-091](../03-adr/ADR-091-el0-isolated-accept-draft.md) sentence.

Archify 3.x no longer renders guided chapters (`meta.views`). The three chapter notes of each diagram now live in the cards under the drawing; the honesty wording above is unchanged. The viewer has built-in Route and Lens controls for reading paths; they are not authored in the JSON.

CI guard: `scripts/check-archify-honesty.py` (workflow `archify-honesty`) fails if an Archify IR still has `meta.views`, or if an honesty label or non-claim phrase (list in `scripts/archify-honesty.config.json`, e.g. `Not FEAT_NMI`, `Not EL0 isolated`) appears in a retired chapter note but not in the components or cards. It also warns when a committed HTML generator string is not `archify 3.2.0`. Self-test: `python3 scripts/test_check_archify_honesty.py`.

### Regenerate

Pin Archify to `v3.2.0`, disable its update check, and render in a scratch directory. `deliver` writes a `*.delivery.json` receipt next to the HTML, and that receipt holds absolute local paths. Copy back only the `.json` and `.html` files; never commit or publish receipts (`.gitignore` lists them).

```bash
# from the repo root
export ARCHIFY_UPDATE_CHECK_DISABLED=1
REPO="$PWD"
git clone --depth 1 --branch v3.2.0 https://github.com/tt-a1i/archify.git /tmp/archify
A=/tmp/archify/archify/bin/archify.mjs
mkdir -p /tmp/archify-out && cp "$REPO"/docs/archify/*.json /tmp/archify-out/ && cd /tmp/archify-out
node "$A" deliver architecture qemu-nmi-taken-serror.architecture.json qemu-nmi-taken-serror.architecture.html
node "$A" deliver sequence taken-serror.sequence.json taken-serror.sequence.html
# optional: node "$A" finalize <type> <input.json> <output.html> --browser-check --out-dir /tmp/archify-receipts/<name>
cp ./*.html ./*.json "$REPO"/docs/archify/   # copies HTML + IR only; leave *.delivery.json behind
```

Edit the `.json`, never the `.html`. Keep the file names: `pages.yml` and `scripts/docs-build.sh` check them.

### Open locally

```bash
xdg-open docs/archify/qemu-nmi-taken-serror.architecture.html
xdg-open docs/archify/taken-serror.sequence.html
```