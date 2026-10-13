# 2026-09-13 — Lower-EL IRQ while standing (ADR-040)

- Tip base: `28d3755` (ADR-039 on main).
- Mile: wire `irq_lower_el`, restore kernel TTBR0, handle GIC, `stay_at_el0` when standing; probe with `WFI` + SPSR I-clear; serial `el0: irq`.
- Still parks: lower-EL FIQ/SError; default standing/task `ERET` still masks IRQ.
- Still Planned: PAN enable, remaining identity RAM / `_start`, umbrella isolation.
- (amd64 workstation) intent `<machine-id>`; CloudAgent HELD — agent-box clone.
