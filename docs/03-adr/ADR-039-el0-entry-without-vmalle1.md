# ADR-039 — EL0 entry / ASID switch without `TLBI VMALLE1`

- Status: Accepted (standing/EL0 trampoline path is `MSR TTBR0` + `ISB` only)
- Date: 2026-09-12

## Context

Track A / A5 isolation leftovers. Dual-ASID EL1 switches already avoided `TLBI VMALLE1` ([ADR-013](ADR-013-el0-isolation-direction.md) P-SEC-3c). The standing/EL0 trampoline still flushed the whole EL1 TLB on every `ERET` to EL0, every stay-at-EL0 after a standing SVC, and every lower-EL sync restore of kernel TTBR0 — because kernel `.data` leaves were **global** (`nG=0`). Dropping that flush while identity `.data` stayed mapped would leak a cached translation into the user ASID and break `el0: no kernel read`.

[ADR-037](ADR-037-identity-data-tear.md) and [ADR-038](ADR-038-identity-heap-tear.md) unmap identity `.data`/`.bss`/stacks and the identity heap (`TLBI VAAE1` each low VA). EL0 probes run **after** those tears. The historical global-`.data` reason for trampoline `VMALLE1` is gone for those ranges. PAN enable and umbrella EL0 isolation are **not** this cut.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Remove `TLBI VMALLE1` from `eret_to_el0`, `stay_at_el0`, and `sync_lower_el` TTBR0 restore. Keep `MSR TTBR0` + `ISB`. Keep `TLBI VMALLE1` only at MMU enable (`paging::init`) and keep `TLBI VAAE1` on unmap/setup. | Smallest honest mile after ADR-037/038. |
| **H2 (rejected)** | Mark all kernel leaves `nG` and keep a full flush "just in case." | Larger surface; not required once identity `.data`/heap are torn + VAAE1'd. |
| **H3 (rejected)** | Claim "EL0 isolated" / PAN / "kernel moved" because the trampoline no longer full-flushes. | Isolation umbrella stays Planned. PAN is [ADR-026](ADR-026-pan-capability.md). |
| **H4 (fallback)** | If H1 regresses `el0: no kernel read` / standing / task smoke, ship a probe documenting where `VMALLE1` still fires and why — do not regress smoke. | Honesty over the mile. |

## Decision

1. **Trampoline path without `VMALLE1`.** `exception::eret_to_el0`, `stay_at_el0`, and `sync_lower_el` (kernel TTBR0 restore from `TPIDR_EL1`) use `MSR TTBR0_EL1` + `ISB` only.
2. **What TLBI remains.** `paging::init` still `TLBI VMALLE1` once at MMU enable. Unmaps / map-window / identity tears still `TLBI VAAE1` (or equivalent per-VA invalidate). ASID dual probe stays no-full-flush ([ADR-013](ADR-013-el0-isolation-direction.md)).
3. **Fail-closed probe.** Serial `el0: no-vmalle1` after the existing EL0 first-mile + read + standing markers, only when identity `.data` and heap tears are ready. `scripts/qemu-smoke.sh` greps that string and rejects `tlbi vmalle1` in `src/exception.rs`. `#[test_case]` `el0_entry_without_vmalle1`.
4. **Keep `_start` / QEMU `-kernel` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN.
5. **Still Planned.** PAN enable on `-cpu cortex-a57`; lower-EL IRQ while standing; remaining identity RAM / `_start` teardown; umbrella EL0 isolation.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.19**. Do not mint NFR-15+.

## Honesty

Say "standing/EL0 trampoline switches TTBR0 without `TLBI VMALLE1` after identity `.data`/heap tear" only when the serial / tests pass. Do **not** say:

- "EL0 isolated"
- PAN (see [ADR-026](ADR-026-pan-capability.md); unimplemented on `-cpu cortex-a57`)
- the kernel has moved / identity mappings were fully torn down (`_start` and leftover identity frames stay)
- "secure OS" / "hardened"

## Consequences

- `src/exception.rs` owns the trampoline TLBI drop. [ADR-013](ADR-013-el0-isolation-direction.md) remains the isolation direction; this ADR is the EL0-entry flush cut.
- A later ADR may enable PAN or tear remaining identity RAM. That work is not this cut.
