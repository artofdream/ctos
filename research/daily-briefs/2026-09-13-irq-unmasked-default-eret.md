# 2026-09-13 — ADR-041 IRQ-unmasked default ERET

- Tip base: `473a73d` (ADR-040 on main).
- Mile: default `eret_to_el0` SPSR `0x340` (I clear); short non-standing probes use `eret_to_el0_masked`; standing IRQ probe uses default ERET (no special helper); serial `el0: irq-default`.
- Probe: agent-box `qemu-smoke` ok (QEMU 10.0.13, nightly `0fc141305`); 89 tests; entry `0x40080000`.
- Still parks: lower-EL FIQ/SError; PAN enable; umbrella isolation; leftover identity RAM / `_start`.
- (evo-x2) intent `f074e48c-…`; CloudAgent HELD — work on agent box clone.
- Do not self-merge.
