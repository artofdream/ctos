# ADR-026 — PAN capability: probe the ID field, do not invent the CPU

- Status: Accepted (capability probe). **PAN enable stays Planned** on the default probe CPU.
- Date: 2026-09-12

## Context

Track A / A5 ([issue #36](https://github.com/artofdream/ctos/issues/36)) may take Privileged Access Never **only** if `ID_AA64MMFR1_EL1.PAN != 0` on the probe CPU **and** an ADR says so. The documented QEMU line is `-cpu cortex-a57` (ARMv8.0). PAN is typically unimplemented there. Silently switching `-cpu` (for example to `max`) to make the field non-zero is not an honest probe.

[ADR-013](ADR-013-el0-isolation-direction.md) already named PAN as a closing isolation probe: an EL1 access to an EL0-accessible page must fault when PSTATE.PAN is set. That enable + fault mile is **not** this ADR.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Read `ID_AA64MMFR1_EL1.PAN` on the default `-cpu`. Print the field. Enable PSTATE.PAN only if the field is non-zero **and** a later cut proves the EL1-vs-EL0 fault. On `cortex-a57` record `pan: absent`. | CPU evidence without inventing hardware. |
| **H2 (rejected)** | Change default `-cpu` to `max` (or another v8.1+ model) so PAN appears. | Silent CPU switch. Forbidden by A5 and by this ADR. |
| **H3 (rejected)** | Claim PAN / “EL0 isolated” from the ID-field print alone. | File presence / a register read is not an access-fault probe. |

## Decision

1. **Probe the ID field.** `src/pan.rs` reads `ID_AA64MMFR1_EL1` bits [23:20] and prints `pan: id=<n>`. It does **not** execute `MSR PAN`. An unimplemented `MSR PAN` is UNDEF on ARMv8.0.
2. **Default CPU stays `cortex-a57`.** `scripts/qemu-serial-inject.py` and `scripts/qemu-aarch64.sh` keep `-cpu cortex-a57`. Do not change them in this mile.
3. **When `n == 0` (expected).** Serial `pan: absent`. Ledger: “PAN enable on virt cortex-a57” stays **Planned**, with this ID-field print as the CPU evidence. `scripts/qemu-smoke.sh` greps `pan: id=` / `pan: absent` and rejects `pan: enabled` / `pan: probe missed`.
4. **When `n != 0`.** Serial `pan: present`. This tree still does **not** enable PSTATE.PAN. A later ADR may `MSR PAN` and prove an EL1 load of an EL0-accessible page faults. Smoke on this repo expects `pan: absent` because the probe CPU is `cortex-a57`.
5. **Still Planned.** PAN enable + EL1-vs-EL0 access fault; lower-EL IRQ while standing; EL0 entry without `TLBI VMALLE1`; umbrella EL0 isolation. [ADR-025](ADR-025-identity-rodata-tear.md) is the identity `.rodata` cut, not this.
6. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “`ID_AA64MMFR1_EL1.PAN` is 0 on `-cpu cortex-a57`” only when the serial / tests pass. Do **not** say:

- PAN is enabled
- EL1 cannot access EL0 pages
- “secure OS” / “hardened” / “EL0 isolated”
- the probe used a different `-cpu` than the scripts

## Consequences

- `src/pan.rs` owns the ID-field probe. Isolation docs cite this ADR as **Planned** for the enable mile.
- A future CPU change needs its own ADR and must not be a silent script edit.
