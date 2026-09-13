# ADR-043 — Lower-EL FIQ while standing (+ SError park honesty)

- Status: Accepted (taken lower-EL FIQ while standing is a supported/probed path when the serial / tests pass; SError has no safe trigger on this virt/cortex-a57 cut — park honesty; umbrella isolation Planned)
- Date: 2026-09-13

## Context

[ADR-040](ADR-040-lower-el-irq-standing.md) / [ADR-041](ADR-041-irq-unmasked-default-eret.md) made taken lower-EL **IRQ** while standing a live path. Lower-EL **FIQ** and **SError** still parked ([ADR-004](ADR-004-el1-vbar-brk.md) / [ADR-042](ADR-042-isolation-leftover-wrap.md) H4).

On QEMU `virt` + GICv2 + `-cpu cortex-a57`, Group 0 interrupts signal as IRQ by default (`GICC_CTLR.FIQEn` clear). Setting FIQEn routes Group 0 (including the CNTP PPI) as FIQ without changing the default `-cpu`. True asynchronous SError has no safe, deterministic inject on this guest without exotic QEMU/monitor setup — do not fake a taken path.

PAN enable and umbrella EL0 isolation are **not** this cut. Do not switch default `-cpu cortex-a57`. Keep `_start` at `0x4008_0000`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Wire `fiq_lower_el` (same TTBR0 restore as IRQ). Probe: temporarily `GICC_CTLR.FIQEn`, standing dual-SVC + `WFI`, SPSR `0x380` (F clear, I set), serial `el0: fiq`; restore Group-0 → IRQ after the trip. SError: wire a defensive standing-aware stub + print `el0: serror-park` (Planned / no safe trigger). | Largest honest first cut mirroring IRQ. |
| **H2 (rejected)** | Claim both FIQ and SError are Verified taken paths without a safe SError inject. | Would round up. |
| **H3 (rejected)** | Switch `-cpu` / enable PAN / claim “EL0 isolated” because FIQ returns to EL0. | Forbidden. |
| **H4 (fallback)** | If FIQEn does not deliver a lower-EL FIQ on this virt guest, ship scaffold + park markers and document why — do not fake Verified. | Honesty over the mile. |

## Decision

1. **Lower-EL FIQ live while standing.** Vector slot `0x500` calls `fiq_lower_el`. `handle_fiq_lower_el` runs `gic::handle_fiq` (same IAR/EOI as IRQ for Group 0), then `stay_at_el0` when `is_active()`. Non-standing lower-EL FIQ still fails closed (park).
2. **Probe.** Hello path: `gic::route_group0_as_fiq`, `timer::arm_soon`, `eret_to_el0_fiq` (SPSR `0x380`), payload `SVC #1` / `WFI` / `MOVZ` / `SVC #2`. Serial `el0: fiq`. Always `gic::route_group0_as_irq` after the trip so the IRQ path stays default.
3. **SError honesty.** Vector slot `0x580` calls `serror_lower_el` (defensive: if standing, stay at EL0 and print `el0: serror`). No taken-path probe on this cut — hello prints `el0: serror-park`. Taken SError stays **Planned**.
4. **Fail-closed sensors.** `scripts/qemu-smoke.sh` greps `el0: fiq` and `el0: serror-park`. `#[test_case]` `lower_el_fiq_while_standing`.
5. **Still Planned.** Taken lower-EL SError; PAN enable on `-cpu cortex-a57`; remaining identity RAM / yank `_start`; umbrella EL0 isolation.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.23**. Do not mint NFR-15+.

## Honesty

Say “taken lower-EL FIQ while standing returned to EL0” only when the serial / tests pass. Do **not** say:

- “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md))
- lower-EL SError is taken / Verified on this guest
- default standing/task `ERET` unmasks FIQ (default SPSR still sets F; only the FIQ probe clears F and arms FIQEn)
- “secure OS” / “hardened”

## Consequences

- `src/gic.rs` owns FIQEn route helpers. `src/exception.rs` owns `fiq_lower_el` / `serror_lower_el`. `src/el0.rs` owns the standing FIQ probe and the SError park marker.
- [ADR-013](ADR-013-el0-isolation-direction.md) remains the isolation direction; this ADR closes the FIQ half of ADR-042 H4.
- A later ADR may take SError safely or enable PAN on a different CPU story. That work is not this cut.
