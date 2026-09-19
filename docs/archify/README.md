# Archify diagrams (ctos)

Documentation aids generated with [artofdream/archify](https://github.com/artofdream/archify). **Not** live production status. Tip reference: `~6ec1f4e` / [ADR-083](../03-adr/ADR-083-b1-qemu-nmi-pin.md).

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

### Regenerate

```bash
node /path/to/archify/bin/archify.mjs deliver architecture \
  docs/archify/qemu-nmi-taken-serror.architecture.json \
  docs/archify/qemu-nmi-taken-serror.architecture.html --quality showcase
node /path/to/archify/bin/archify.mjs deliver sequence \
  docs/archify/taken-serror.sequence.json \
  docs/archify/taken-serror.sequence.html --quality showcase
```
