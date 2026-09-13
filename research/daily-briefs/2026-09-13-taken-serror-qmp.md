# 2026-09-13 — Taken SError QMP attempt (ADR-045)

- Guest: `eret_to_el0_serror` (SPSR `0x2c0`, A clear) + standing spin + `el0: serror-arm` cue.
- Host: `qemu-serial-inject.py` QMP `inject-nmi` after cue.
- Result on QEMU 10.0.13 virt+cortex-a57: `machine does not provide NMIs` — taken path **Planned**; `el0: serror-park` stays Verified.
- No `#[test_case]` without a working host inject. (evo-x2) intent; CloudAgent HELD.
