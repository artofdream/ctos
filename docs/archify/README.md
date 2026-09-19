# Archify diagrams (ctos)

Documentation aids generated with [artofdream/archify](https://github.com/artofdream/archify). **Not** live production / umbrella status. Tip reference: post-[ADR-083](../03-adr/ADR-083-b1-qemu-nmi-pin.md).

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
- Not FEAT_NMI / FIQ / BRK as SError; not umbrella EL0 isolated.

### Open locally

```bash
xdg-open docs/archify/qemu-nmi-taken-serror.architecture.html
xdg-open docs/archify/taken-serror.sequence.html
```