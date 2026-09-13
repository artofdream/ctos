# ADR-041 — IRQ-unmasked default standing/task ERET

- Status: Accepted (default `ERET` to EL0 clears SPSR.I when the serial / tests pass; short non-standing trampoline probes stay masked; FIQ/SError lower-EL still park; umbrella isolation Planned)
- Date: 2026-09-13

## Context

[ADR-040](ADR-040-lower-el-irq-standing.md) made taken lower-EL IRQ while standing a live path, but only via a dedicated probe SPSR (`eret_to_el0_irq_enabled`, I clear). Default `eret_to_el0` still used SPSR `0x3c0` (I set), so standing/task entry remained IRQ-masked unless that special trampoline was used.

This cut makes standing EL0 and the loaded-task / standing-payload paths enter with IRQ **unmasked by default**, and proves a timer tick can arrive without the special probe helper. PAN enable and umbrella EL0 isolation are **not** this cut. Do not switch default `-cpu cortex-a57`. Keep `_start` at `0x4008_0000`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Default `eret_to_el0` uses SPSR `0x340` (EL0t, I clear). Short non-standing trampoline probes (`SVC` / IABORT / DABORT without `is_active()`) use `eret_to_el0_masked`. Standing IRQ probe uses **default** `eret_to_el0` + `WFI` + `arm_soon` (no special SPSR helper). Serial `el0: irq` + `el0: irq-default`. Task/loader/syscall/libctos standing paths inherit the default. FIQ/SError stay parked. | Largest honest cut that matches the deferred ADR-040 item. |
| **H2 (rejected)** | Unmask every `ERET` including short non-standing probes and claim “IRQ mid-SVC is always fine.” | Non-standing lower-EL IRQ still parks (`handle_irq_lower_el`); a mid-probe tick would fail-closed. |
| **H3 (rejected)** | Claim “EL0 isolated” / PAN / “secure OS” because default SPSR clears I. | Isolation umbrella stays Planned. |
| **H4 (fallback)** | If H1 regresses smoke, keep default masked and document the park — do not regress smoke. | Honesty over the mile. |

## Decision

1. **Default `ERET` IRQ-unmasked.** `eret_to_el0` writes SPSR `0x340`. `eret_to_el0_irq_enabled` becomes an alias (or is removed). `eret_to_el0_masked` keeps SPSR `0x3c0` for short non-standing probes.
2. **Probe.** Hello path: same standing dual-SVC + `WFI` as ADR-040, but enter via **default** `eret_to_el0`. Serial still `el0: irq`; also print `el0: irq-default` after that trip succeeds. Existing `el0: standing` / `el0: restored` still print.
3. **Task path.** Loaded standing task / A1–A3 standing payloads use `eret_to_el0` and therefore inherit I-clear. No separate task WFI probe required for this cut; IRQ while `is_active()` already returns via `stay_at_el0`.
4. **Fail-closed sensors.** `scripts/qemu-smoke.sh` greps `el0: irq-default`. `#[test_case]` `lower_el_irq_default_eret` (or extend `lower_el_irq_while_standing` to assert default path).
5. **Still Planned.** Lower-EL FIQ/SError; PAN enable on `-cpu cortex-a57`; remaining identity RAM / `_start` teardown; umbrella EL0 isolation.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.21**. Do not mint NFR-15+.

## Honesty

Say “default standing/task `ERET` enters with IRQ unmasked” only when the serial / tests pass. Do **not** say:

- “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md))
- lower-EL FIQ/SError are handled
- short non-standing trampoline probes take IRQs (those stay masked)
- “secure OS” / “hardened”

## Consequences

- `src/exception.rs` owns SPSR defaults. `src/el0.rs` owns the default-path IRQ probe. Short probes in `el0` / `ttbr1` / `teardown` call the masked helper.
- [ADR-013](ADR-013-el0-isolation-direction.md) remains the isolation direction; this ADR closes the ADR-040 “default ERET still masks I” park.
- A later ADR may take FIQ/SError or enable PAN. That work is not this cut.
