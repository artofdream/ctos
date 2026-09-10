# EL0 isolation (P-SEC-3 / ADR-013)

**Status: Planned.** This page is a scaffold. EL0 is not implemented. Do not claim userspace or isolation.

Direction: [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md). Threat model: [security.md](security.md). Stub: `src/el0.rs` (`is_active() == false`).

## What exists today

- Kernel runs at EL1 (`SPSel = 0`).
- Lower-EL vector slots park ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)).
- RAM maps set UXN so a *future* EL0 cannot fetch those pages; that is not an EL0 probe (no EL0 to take the fault).

## Planned probes (not built)

| Probe | What would close it |
| --- | --- |
| Lower-EL cannot execute kernel data | Serial marker + `#[test_case]`: EL0 (or a lower-EL fetch) of a kernel page faults (permission / translation). |
| Lower-EL slots are taken, not only parked | A deliberate `ERET` to EL0 that returns via SVC or faults observably. |
| User map ≠ kernel map | A user VA that is not the kernel identity window. |

Unprobed stays **Unknown**. Planned stays **Planned** until the code and the probe exist.
