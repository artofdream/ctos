# ADR-054 — PAN enable lock on default `-cpu cortex-a57` (reopen gate)

- Status: Accepted (docs / evidence lock). Reopen gate items **1–4** are **met**: [ADR-079](ADR-079-pan-cpu-reopen.md) (CPU + `pan: present`) and [ADR-080](ADR-080-pan-enable-fault.md) (enable + EL1-vs-EL0 fault). Historical a57 ID-field stays **Verified** (`pan: absent`, [ADR-026](ADR-026-pan-capability.md)). Do **not** claim “EL0 isolated.”
- Date: 2026-09-14
- Locks [ADR-047](ADR-047-isolation-leftovers-decisions.md) decision #2 with an explicit reopen gate. Does not change guest code or default `-cpu`.

## Context

[ADR-026](ADR-026-pan-capability.md) Verified that `ID_AA64MMFR1_EL1.PAN == 0` on `-cpu cortex-a57` (serial `pan: id=0` / `pan: absent`). Enable + EL1-vs-EL0 access fault is impossible without inventing hardware or silently switching `-cpu`. [ADR-047](ADR-047-isolation-leftovers-decisions.md) already called enable a **non-goal** on the default probe CPU. Sponsor / DSO asked to **secure** this gap before Guest Linux / containers / immutable-OS marketing: ADR+evidence lock so enable is not a fuzzy Planned mile, with an explicit reopen gate.

Do **not** silent `-cpu` switch. Do **not** execute `MSR PAN` while the ID field is 0. Do **not** claim “EL0 isolated.” CloudAgent HELD; docs-only.

## Evidence (locked)

| Evidence | Status | Cite |
| --- | --- | --- |
| Default smoke CPU is `-cpu cortex-a57` | Scripts contract | `scripts/qemu-aarch64.sh`, `scripts/qemu-serial-inject.py` |
| `ID_AA64MMFR1_EL1.PAN == 0` → `pan: absent` | **Verified** | [ADR-026](ADR-026-pan-capability.md); honesty-ledger PAN ID-field rows (e.g. 2026-09-12 `pan: id=0` / `pan: absent`) |
| Smoke rejects `pan: enabled` | **Verified** harness | `scripts/qemu-smoke.sh` |
| QEMU CPU model props do not expose a free “turn on PAN” on cortex-a57 without a model change | Agent-box QMP 2026-09-14 | `query-cpu-model-expansion` `type=full` `model.name=cortex-a57` — no `pan` prop to flip on this model |

## Decision

1. **Lock.** PAN **enable** (PSTATE.PAN / `MSR PAN` + EL1 load of an EL0-accessible page must fault) is a **non-goal** on the default probe CPU `-cpu cortex-a57`. Roadmap P-SEC-3k stays **Non-goal (default probe CPU)**, not Planned.
2. **ID-field stays Verified.** Keep printing `pan: id=` / `pan: absent`. Keep `#[test_case]` `pan_unimplemented_on_probe_cpu`. Smoke must still reject `pan: enabled` / `pan: probe missed`.
3. **Explicit reopen gate.** PAN enable may be reopened **only** when **all** of the following hold:
   1. **Sponsor-approved** new default probe CPU (or documented FEAT_PAN-present CPU story) — recorded in a **new ADR**, not a silent script edit.
   2. Serial (or equivalent) evidence that `ID_AA64MMFR1_EL1.PAN != 0` on that accepted CPU (`pan: present`).
   3. A follow-up ADR that enables PSTATE.PAN and proves an EL1-vs-EL0 access fault (fail-closed if the fault does not land).
   4. Honesty ledger + smoke greps updated for the new CPU; old a57 `pan: absent` rows remain historical truth for a57.
4. Until the reopen gate is met, do **not** claim PAN, “privileged access never,” or “EL0 isolated.”


## Reopen progress (2026-09-19)

[ADR-079](ADR-079-pan-cpu-reopen.md) / [#125](https://github.com/artofdream/ctos/issues/125): sponsor unlock + default probe CPU `-cpu cortex-a76` + serial `pan: present` (gate items 1–2). [ADR-080](ADR-080-pan-enable-fault.md): enable + EL1-vs-EL0 fault (gate items 3–4). a57 `pan: absent` remains historical truth for that CPU. Umbrella “EL0 isolated” still non-claim ([ADR-055](ADR-055-el0-isolated-checklist.md)).

## Honesty

Say “PAN enable is locked non-goal on default `-cpu cortex-a57`; ID-field Verified `pan: absent` (ADR-026).” Do **not** say:

- PAN is enabled
- EL1 cannot access EL0 pages (PAN mile)
- the probe used `-cpu max` / another model without a sponsor ADR
- “EL0 isolated” / “secure OS”

## Consequences

- Ledger / roadmap P-SEC-3k cite this ADR as the **enable lock** + reopen gate.
- [ADR-026](ADR-026-pan-capability.md) remains the capability probe; this ADR is the enable hard-lock.
- [ADR-055](ADR-055-el0-isolated-checklist.md) lists PAN enable as an unmet umbrella requirement.
- Threat-model patch bump. No `src/` change. No silent `-cpu` edit.
