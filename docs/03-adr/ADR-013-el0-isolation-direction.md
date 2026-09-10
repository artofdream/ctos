# ADR-013 — EL0 isolation direction (first mile)

- Status: Accepted (direction + first mile; isolation still Planned)
- Date: 2026-09-10
- Updated: 2026-09-10 (post-#18 deepen)

## Context

Vision and [FR-11](../02-requirements/fr-nfr.md) leave userspace / EL0 as later work. [NFR-10](../02-requirements/fr-nfr.md) names EL0 isolation as Planned. [ADR-011](ADR-011-three-pillars.md) and the threat model ([security.md](../framework/security.md)) treat “malicious EL0” as a **named** adversary, not a standing userspace.

The first revision of this ADR recorded direction only (`is_active() == false`, lower-EL slots parked). The post-#18 deepen implements the **smallest honest mile** toward the closing probe. It does not invent a POSIX process model.

## Decision

1. **First mile (this tree).** Deliberate `ERET` to EL0t (DAIF masked) on one map-window page that is UXN-clear and PXN (EL1 must not fetch it). Two paths:
   - `SVC #0` taken on the lower-EL AArch64 **sync** slot; handler prints `el0: svc` and `ERET`s back to EL1t.
   - `BR X0` to a kernel `.data` bait; UXN on the identity image must take a lower-EL permission IABORT; handler prints `el0: nx kernel` and returns to EL1t.
   Serial `el0: ok` after both. `is_active()` stays **false** — there is no standing EL0 context.
2. **Closing probe for “cannot execute kernel data.”** The UXN IABORT *is* that instruction-fetch probe. Mark that **mile** Verified when the serial / `#[test_case]` pass. Do **not** mark isolation Verified: `TTBR0` is still shared, there is no PAN, no ASID, and EL0 can still *read* kernel pages.
3. **Isolation remains Planned.** Later ingredients (each its own ADR when started): user vs kernel `TTBR0` (or a dedicated user window that is not the kernel identity map), ASID, PAN, a real user stack, more than one SVC. Do not move FR IDs. Do not mint FR-16+.
4. **Lower-EL slots.** AArch64 sync is live for the paths above. Lower-EL IRQ / FIQ / SError and all AArch32 slots stay parks ([ADR-004](ADR-004-el1-vbar-brk.md)).
5. **Honesty.** Docs and PRs may say “EL0 entered and returned” or “EL0 cannot execute kernel data” when those probes pass. They may not say “EL0 works,” “userspace,” or “isolated.”

## Consequences

- [el0.md](../framework/el0.md) splits first-mile vs isolation rows.
- Preemption and SMP remain separate later ADRs ([ADR-010](ADR-010-cooperative-rr-el1.md)).
- This ADR does not claim Raspberry Pi or a POSIX process model.
