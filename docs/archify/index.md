# Archify diagrams

Same-origin views on this site (Documented aid — **not** Live product status; umbrella "EL0 isolated" stays non-claim).

| Diagram | Open |
| --- | --- |
| QEMU NMI pin vs stock park (architecture) | [HTML](/archify/qemu-nmi-taken-serror.architecture.html) · [JSON](/archify/qemu-nmi-taken-serror.architecture.json) |
| Taken SError inject sequence | [HTML](/archify/taken-serror.sequence.html) · [JSON](/archify/taken-serror.sequence.json) |

## Honesty

- Taken `el0: serror` is **Verified opt-in** (`CTOS_QEMU` + `CTOS_REQUIRE_TAKEN_SERROR=1`); see [ADR-083](../03-adr/ADR-083-b1-qemu-nmi-pin.md).
- Stock distro QEMU still parks (`el0: serror-park`).
- Not FEAT_NMI / FIQ / BRK as SError; not "EL0 isolated."

Source folder: `docs/archify/` (copied into the published `book/archify/` on Pages).