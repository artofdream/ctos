# 2026-09-13 — Taken SError research (ADR-044)

- Docs-only. Taken lower-EL SError stays **Planned**.
- Recommended next mile: QEMU QMP/`nmi` (virt→`ARM_CPU_SERROR`) while standing with `PSTATE.A` clear.
- Guest-only RAS inject rejected for Verified on `-cpu cortex-a57` TCG.
- Park marker `el0: serror-park` remains the Verified honesty line (ADR-043).