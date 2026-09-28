# Archify diagrams

Same-origin views on this site (Documented aid â€” **not** Live product status; snapshot from before M6; for “EL0 isolated” see the accepted [ADR-091](../03-adr/ADR-091-el0-isolated-accept-draft.md) sentence).

| Diagram | Open |
| --- | --- |
| QEMU NMI pin vs stock park (architecture) | [HTML](/archify/qemu-nmi-taken-serror.architecture.html) Â· [JSON](/archify/qemu-nmi-taken-serror.architecture.json) |
| Taken SError inject sequence | [HTML](/archify/taken-serror.sequence.html) Â· [JSON](/archify/taken-serror.sequence.json) |

## Honesty

- Taken `el0: serror` is **Verified opt-in** (`CTOS_QEMU` + `CTOS_REQUIRE_TAKEN_SERROR=1`); see [ADR-083](../03-adr/ADR-083-b1-qemu-nmi-pin.md).
- Stock distro QEMU still parks (`el0: serror-park`).
- Not FEAT_NMI / FIQ / BRK as SError. The pin alone is not the umbrella; since M6, “EL0 isolated” is only the accepted [ADR-091](../03-adr/ADR-091-el0-isolated-accept-draft.md) sentence.

Source folder: `docs/archify/` (copied into the published `book/archify/` on Pages).

## Related

- [Chronify evolution rail](../chronify/index.md) — dated milestones (Documented until Pages probe).

