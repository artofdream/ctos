# ADR-040 — Lower-EL IRQ while standing EL0

- Status: Accepted (taken lower-EL IRQ while standing is a supported/probed path when the serial / tests pass; FIQ/SError lower-EL still park; umbrella isolation Planned)
- Date: 2026-09-13

## Context

Track A / A5 isolation leftovers. Lower-EL AArch64 **sync** has long been live for SVC / IABORT / DABORT. The lower-EL **IRQ** / FIQ / SError vector slots historically parked ([ADR-004](ADR-004-el1-vbar-brk.md)). Standing EL0 ([ADR-013](ADR-013-el0-isolation-direction.md) / [ADR-024](ADR-024-standing-el0-normal.md)) entered with DAIF IRQ masked in SPSR, so a timer IRQ from EL0 was never taken.

After [ADR-039](ADR-039-el0-entry-without-vmalle1.md), the standing/EL0 trampoline restores kernel TTBR0 with `MSR TTBR0` + `ISB` only. The same restore is required on a lower-EL IRQ path before GIC / `.data` access (user TTBR0 omits MMIO and kernel data).

PAN enable and umbrella EL0 isolation are **not** this cut. Do not switch default `-cpu cortex-a57`. Keep `_start` at `0x4008_0000`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Wire lower-EL IRQ (`irq_lower_el`): restore kernel TTBR0, `gic::handle_irq`, if `el0::is_active()` put user TTBR0 back and `ERET` to EL0. Probe: standing dual-SVC with `WFI` and SPSR IRQ-unmasked; serial `el0: irq`. Default `ERET` to EL0 stays IRQ-masked. FIQ/SError stay parked. | Largest honest first cut that does not race existing EL0 probes. |
| **H2 (rejected)** | Unmask IRQ on every standing/task `ERET` and claim “IRQ while standing is normal for all tasks.” | Larger surface; timer mid-task may interact with A4/A1–A3 probes before this mile is proven. |
| **H3 (rejected)** | Claim “EL0 isolated” / PAN / “secure OS” because IRQ returns to EL0. | Isolation umbrella stays Planned. |
| **H4 (fallback)** | If H1 regresses smoke, ship a scaffold documenting the park and why — do not regress smoke. | Honesty over the mile. |

## Decision

1. **Lower-EL IRQ live while standing.** Vector slot `0x480` calls `irq_lower_el` (same frame + TTBR0 restore as `sync_lower_el`). `handle_irq_lower_el` runs `gic::handle_irq`, then `stay_at_el0` when `is_active()`. Non-standing lower-EL IRQ still fails closed (park).
2. **Probe.** Hello path: arm standing + expect IRQ, `timer::arm_soon`, `ERET` with SPSR `0x340` (EL0t, I clear), payload `SVC #1` / `WFI` / `MOVZ` / `SVC #2`. Serial `el0: irq` when the IRQ was taken while standing. Existing `el0: standing` / `el0: restored` still print on that trip.
3. **Default ERET stays masked.** `eret_to_el0` keeps SPSR `0x3c0`. Only the IRQ probe uses `eret_to_el0_irq_enabled`. Enabling IRQ for all standing tasks is a later cut.
4. **Fail-closed sensors.** `scripts/qemu-smoke.sh` greps `el0: irq`. `#[test_case]` `lower_el_irq_while_standing`.
5. **Still Planned.** Lower-EL FIQ/SError; IRQ-unmasked default standing/task `ERET`; PAN enable on `-cpu cortex-a57`; remaining identity RAM / `_start` teardown; umbrella EL0 isolation.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.20**. Do not mint NFR-15+.

## Honesty

Say “taken lower-EL IRQ while standing returned to EL0” only when the serial / tests pass. Do **not** say:

- “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md))
- lower-EL FIQ/SError are handled
- every standing task takes IRQs by default (default `ERET` still masks I)
- “secure OS” / “hardened”

## Consequences

- `src/exception.rs` owns `irq_lower_el` / `handle_irq_lower_el`. `src/el0.rs` owns the `WFI` standing probe. `src/timer.rs` owns `arm_soon`.
- [ADR-013](ADR-013-el0-isolation-direction.md) remains the isolation direction; this ADR is the lower-EL IRQ-while-standing cut.
- A later ADR may unmask IRQ for standing tasks by default, or take FIQ/SError. That work is not this cut.
