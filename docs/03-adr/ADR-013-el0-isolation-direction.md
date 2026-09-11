# ADR-013 — EL0 isolation direction (first mile)

- Status: Accepted (direction + first mile + user-TTBR0 read mile; isolation still Planned)
- Date: 2026-09-10
- Updated: 2026-09-11 (user TTBR0 window; isolation still Planned)

## Context

Vision and [FR-11](../02-requirements/fr-nfr.md) leave userspace / EL0 as later work. [NFR-10](../02-requirements/fr-nfr.md) names EL0 isolation as Planned. [ADR-011](ADR-011-three-pillars.md) and the threat model ([security.md](../framework/security.md)) treat “malicious EL0” as a **named** adversary, not a standing userspace.

The first revision of this ADR recorded direction only (`is_active() == false`, lower-EL slots parked). The post-#18 deepen implements the **smallest honest mile** toward the closing probe. It does not invent a POSIX process model.

## Decision

1. **First mile (this tree).** Deliberate `ERET` to EL0t (DAIF masked) on one map-window page that is UXN-clear and PXN (EL1 must not fetch it). Two paths:
   - `SVC #0` taken on the lower-EL AArch64 **sync** slot; handler prints `el0: svc` and `ERET`s back to EL1t.
   - `BR X0` to a kernel `.data` bait; UXN on the identity image must take a lower-EL permission IABORT; handler prints `el0: nx kernel` and returns to EL1t.
   Serial `el0: ok` after both. `is_active()` stays **false** — there is no standing EL0 context.
2. **Closing probe for “cannot execute kernel data.”** The UXN / translation IABORT *is* that instruction-fetch probe. Mark that **mile** Verified when the serial / `#[test_case]` pass.
3. **User TTBR0 window (this tree, post-#19).** A second L1 (`L1_USER`) is installed on `ERET` to EL0 (ASID=1 in TTBR0[63:48]). It maps kernel `.text`/`.rodata` and the exception stack so the lower-EL handler can restore kernel TTBR0 (kept in `TPIDR_EL1`), plus the shared map-window L2 for the trampoline. It **omits** `.data` / `.bss` / heap. An EL0 `LDR` from kernel `.data` is a lower-EL translation (or permission) DABORT (`el0: no kernel read`). We still `TLBI VMALLE1` on the switch — ASID is programmed, not proven as TLB isolation.
4. **Isolation remains Planned.** Missing: standing EL0 (`is_active()` stays false), PAN (typically unimplemented on `-cpu cortex-a57` / ARMv8.0 — read `ID_AA64MMFR1_EL1.PAN` before claiming it), ASID-tagged TLB isolation without a full invalidate, TTBR1 / higher-half, more than one SVC. Do not move FR IDs. Do not mint FR-16+.
5. **Lower-EL slots.** AArch64 sync is live for SVC / IABORT / DABORT. Lower-EL IRQ / FIQ / SError and all AArch32 slots stay parks ([ADR-004](ADR-004-el1-vbar-brk.md)).
6. **Honesty.** Docs and PRs may say “EL0 entered and returned,” “EL0 cannot execute kernel data,” “EL0 cannot read kernel `.data`,” or “user TTBR0 omits kernel data” when those probes pass. They may not say “EL0 works,” “userspace,” or “isolated.”

## Consequences

- [el0.md](../framework/el0.md) splits first-mile vs isolation rows.
- Preemption and SMP remain separate later ADRs ([ADR-010](ADR-010-cooperative-rr-el1.md)).
- This ADR does not claim Raspberry Pi or a POSIX process model.
