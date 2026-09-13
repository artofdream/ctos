# ADR-047 — Isolation leftovers decisions (SError / PAN / `_start` / umbrella)

- Status: Accepted (docs decision). Does **not** mint Verified for taken SError, PAN enable, yank `_start`, or “EL0 isolated.”
- Date: 2026-09-13

## Context

Track A / A5 closed a deep isolation ladder through [ADR-042](ADR-042-isolation-leftover-wrap.md) (start-stay honesty), [ADR-043](ADR-043-lower-el-fiq-serror.md) (FIQ + SError park), [ADR-044](ADR-044-taken-serror-research.md) / [ADR-045](ADR-045-taken-serror-qmp.md) (taken SError research + QMP attempt), and [ADR-026](ADR-026-pan-capability.md) (PAN ID-field). Four leftovers still read as open **Planned** work in the ledger and roadmap even though sponsor intent is to **decide** them honestly rather than leave them as pending miles that invite a fake Verified.

This ADR records those decisions. It does not change guest code. CloudAgent HELD; (evo-x2) Docker re-smoke is optional for this docs-only PR.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Decide each leftover: taken SError = non-goal/deferred on this smoke machine; PAN enable = non-goal on default `-cpu cortex-a57`; never yank `_start` while `-kernel` needs `0x4008_0000`; leftover frame RAM optional Planned; umbrella “EL0 isolated” stays Planned/non-claim. | Honest close of the leftover *decision* without inventing probes. |
| **H2 (rejected)** | Mark taken SError / PAN enable / “EL0 isolated” Verified from existing miles. | Round-up. Forbidden. |
| **H3 (rejected)** | Silent `-cpu` switch or yank `_start` to “finish” isolation. | Breaks honesty / QEMU `-kernel`. |

## Decision

### 1. Taken lower-EL SError — non-goal / deferred on this smoke machine

- QMP `inject-nmi` fails on QEMU 10 `virt` + `-cpu cortex-a57` (`machine does not provide NMIs`). There is no honest host inject on this probe today ([ADR-045](ADR-045-taken-serror-qmp.md)).
- **Decision:** taken lower-EL SError while standing is a **non-goal / deferred** on this smoke machine until an honest inject exists (new QEMU feature, accepted different machine story, or sponsor-approved CPU/GIC change with its own ADR).
- Keep `el0: serror-park` **Verified** (park honesty).
- Keep guest A-clear standing plumbing (`el0: serror-arm` / `eret_to_el0_serror` / EXPECT) as **dormant prep** — do not delete; do not require `el0: serror` in smoke.
- Do **not** claim taken SError Verified.

### 2. PAN enable on default `-cpu cortex-a57` — non-goal on the default probe CPU

- ID field is 0 (`pan: absent`). Enable + EL1-vs-EL0 fault is impossible without inventing hardware ([ADR-026](ADR-026-pan-capability.md)).
- **Decision:** **PAN enable is a non-goal on the default probe CPU.** Do not silent `-cpu` switch.
- ID-field probe (`pan: id=` / `pan: absent`) stays **Verified**.
- An optional future CPU story (different default `-cpu`, FEAT_PAN present, enable + fault probe) needs its **own ADR + sponsor approval**. Until then, do not claim PAN.

### 3. Yank `_start` / leftover identity RAM

- QEMU `-kernel` still needs `_start` at `0x4008_0000`.
- **Decision:** **never yank `_start`** while that boot contract holds. `ident: start-stay` / `ident: ram-stay` stay **Verified honesty** ([ADR-042](ADR-042-isolation-leftover-wrap.md)).
- Leftover identity frame RAM after the heap may stay **Planned** as an **optional** later unmap. It is **not required** to close the umbrella isolation row under current constraints.
- Do not claim “the kernel moved” or full identity teardown.

### 4. Umbrella “EL0 isolated” (P-SEC-3l) — Planned / non-claim

- **Decision:** remains **Planned / non-claim** under current constraints (no PAN enable on a57, `_start` stays, taken SError deferred).
- Specific Verified miles (IRQ/FIQ/tears/start-stay/PAN ID-field/…) are **not** this row. Do not round them up to “EL0 isolated.”

## Status table (after this ADR)

| Item | Status | Notes |
| --- | --- | --- |
| `el0: serror-park` | **Verified** | Park honesty |
| Guest A-clear SError prep (ADR-045) | **Dormant prep** | Not a taken probe |
| Taken lower-EL SError on this smoke machine | **Deferred / non-goal** | Until honest inject |
| PAN ID-field on cortex-a57 | **Verified** | `pan: absent` |
| PAN enable on default cortex-a57 | **Non-goal** (default probe CPU) | Future CPU = new ADR + sponsor |
| `ident: start-stay` / never yank `_start` | **Verified (honesty)** / **decided** | Keep `0x4008_0000` |
| Leftover identity RAM after heap | **Verified (optional)** via [ADR-049](ADR-049-identity-ram-tear.md) | Not umbrella-required |
| Umbrella “EL0 isolated” | **Planned / non-claim** | Constraints above |

## Honesty

Say the decisions above only as documentation. Do **not** say:

- taken lower-EL SError is Verified
- PAN is enabled / “EL0 isolated”
- `_start` was yanked / the kernel moved
- app hosting is done (see [ADR-048](ADR-048-app-hosting-claim-criteria.md))

## Consequences

- Docs: [el0.md](../framework/el0.md), [security.md](../framework/security.md) (threat-model **v1.26**), honesty ledger (Planned→decided wording), roadmap P-SEC-3k/3l/3r + leftover blurb, NFR-10 text in place, SUMMARY.
- Code: unchanged. Smoke still requires park / start-stay / `pan: absent`; still rejects `el0: serror` as a required marker and rejects `pan: enabled`.
- A later ADR may reopen taken SError (honest inject), PAN (new CPU story), or boot redesign so `_start` can leave identity — each needs sponsor scope.
