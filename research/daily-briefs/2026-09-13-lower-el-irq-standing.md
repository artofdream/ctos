# 2026-09-13 — Lower-EL IRQ while standing (ADR-040)

- Tip base: `28d3755` (ADR-039 on main).
- Mile: wire `irq_lower_el`, restore kernel TTBR0, handle GIC, `stay_at_el0` when standing; probe with `WFI` + SPSR I-clear; serial `el0: irq`.
- Still parks: lower-EL FIQ/SError; default standing/task `ERET` still masks IRQ.
- Still Planned: PAN enable, remaining identity RAM / `_start`, umbrella isolation.
- (evo-x2) intent `f074e48c-ff1d-4a00-945c-74838a5b1550`; CloudAgent HELD — agent-box clone.
