# ADR-053 — Taken lower-EL SError hard-stop (no honest inject on smoke machine)

- Status: Accepted (docs / evidence lock). Taken lower-EL SError while standing stays **deferred / non-goal** and is **locked**. Reopen re-probe on default `-cpu cortex-a76`: [ADR-081](ADR-081-taken-serror-reopen.md) / [ADR-082](ADR-082-b1-qemu-type-nmi.md) (2026-09-19) — still blocked. (not a fuzzy Planned mile). `el0: serror-park` remains **Verified**. Guest A-clear plumbing remains **dormant prep**. Do **not** mark taken SError Verified.
- Date: 2026-09-14
- Supersedes fuzzy Planned wording for this row; does **not** reopen [ADR-047](ADR-047-isolation-leftovers-decisions.md) decision #1 — it hard-stops it with dated re-probe evidence.

## Context

[ADR-044](ADR-044-taken-serror-research.md) recommended host QMP/`inject-nmi` (H1). [ADR-045](ADR-045-taken-serror-qmp.md) wired the guest A-clear standing path and attempted H1; QEMU 10 `virt` + `-cpu cortex-a57` returned `machine does not provide NMIs`. [ADR-047](ADR-047-isolation-leftovers-decisions.md) decided deferred/non-goal. Sponsor / DSO asked to **secure** this gap before Guest Linux / containers / immutable-OS marketing: re-check whether any honest inject exists; if not, ADR hard-stop with dated evidence so the taken path is not a fuzzy Planned mile.

Do **not** silent `-cpu` switch. Do **not** claim “EL0 isolated.” Keep `_start` at `0x4008_0000`. Do **not** unmask A on default standing/task `ERET`. CloudAgent HELD; docs-only (no new guest inject code — no honest host path exists).

## Dated re-probe (2026-09-14, agent box)

Smoke machine (same class as ADR-045 / ledger):

- Host: agent box
- `qemu-system-aarch64` **10.0.13** (Debian `1:10.0.13+ds-0+deb13u1`)
- `-machine virt` + `-cpu cortex-a57`
- QMP unix socket; guest paused with `-S` (machine/CPU identity only — inject availability does not need the ctos kernel)

Commands tried (documented H1 path only; no invented inject):

| Attempt | Result |
| --- | --- |
| QMP `{"execute":"inject-nmi"}` | **error** `class=GenericError` `desc=machine does not provide NMIs` |
| QMP `human-monitor-command` / HMP `nmi` | **Error: machine does not provide NMIs** |

`query-commands` on this QEMU shows `inject-nmi` present, but the virt machine does **not** implement `TYPE_NMI` for this CPU/GIC story. Other `*-inject-*` commands are CXL-only and are **not** an ARM SError path for ctos. ADR-044 alternatives remain rejected or out of scope:

| ID | Option | 2026-09-14 status |
| --- | --- | --- |
| H1 | QMP/HMP `nmi` → `ARM_CPU_SERROR` | **Impossible** on this smoke machine (dated error text above). Matches ADR-045 / evo-x2 `3f7d3c5` smoke log. |
| H2 | Guest-only RAS / poisoned load | Still **rejected for Verified** (ADR-044). |
| H3 | Treat sync/BRK/FIQ as SError | Still **forbidden**. |
| H5 | KVM `serror_pending` / plugin inject | Still **out of scope** for TCG virt `-kernel` smoke. |

Modern virt “NMI” / FEAT_NMI (GICv3 + `aa64_nmi`) is a **different** CPU/GIC story and would require a silent or sponsor-approved `-cpu` / GIC change — not an honest inject on the default probe.

## Decision

1. **Hard-stop.** Taken lower-EL SError while standing on QEMU 10 `virt` + `-cpu cortex-a57` is **deferred / non-goal** and **locked**. It is **not** a Planned implementation mile awaiting a “try harder” cut on this machine.
2. **Park stays Verified.** Smoke must keep requiring `el0: serror-park`. Do **not** require `el0: serror`.
3. **A-clear plumbing stays dormant prep.** Keep `el0: serror-arm` / `eret_to_el0_serror` / EXPECT from ADR-045. Do not delete; do not claim taken.
4. **Reopen gate (only).** A later cut may reopen taken SError **only** with: (a) an honest host inject that actually delivers SError on the **accepted** smoke machine, **or** a sponsor-approved machine/CPU/GIC change with its own ADR; (b) serial `el0: serror` under EXPECT; (c) a new ADR that cites working evidence. Until then, do not mint Verified. **2026-09-19 re-probe:** [ADR-081](ADR-081-taken-serror-reopen.md) / [ADR-082](ADR-082-b1-qemu-type-nmi.md) — gate unmet on virt+cortex-a76 (and GICv3 / virtualization / max variants); explicit unlock = ADR-081 Decision §4 B1/B2/B3 (B1 research: ADR-082).
5. **Still non-claims:** PAN enable on default a57 ([ADR-054](ADR-054-pan-enable-lock.md)); umbrella “EL0 isolated” ([ADR-055](ADR-055-el0-isolated-checklist.md)); never yank `_start`.

## Honesty

Say “taken lower-EL SError is hard-stopped / deferred on this smoke machine; QMP `inject-nmi` still returns `machine does not provide NMIs` (re-probed 2026-09-14).” Do **not** say:

- taken lower-EL SError is Verified
- “EL0 isolated” / PAN enabled / “secure OS”
- a different `-cpu` was used to make inject work
- guest-only RAS inject is Verified
- the taken path is still a near-term Planned mile on virt+a57

## Follow-on

[ADR-083](ADR-083-b1-qemu-nmi-pin.md) (2026-09-19): pinned QEMU opt-in Verifies taken `el0: serror`; stock park path unchanged.

## Consequences

- Ledger / roadmap P-SEC-3r: cite this ADR as the **hard-stop lock** (park Verified; taken locked deferred/non-goal).
- [ADR-045](ADR-045-taken-serror-qmp.md) remains the implementation attempt; this ADR is the dated impossibility lock.
- [ADR-047](ADR-047-isolation-leftovers-decisions.md) decision #1 stands; wording tightens from “until an honest inject exists” (open-ended) to “locked until reopen gate in Decision §4.”
- Threat-model patch version bump ([security.md](../framework/security.md)). No `src/` change. No new FR/NFR IDs.
- Reopen attempt: [ADR-081](ADR-081-taken-serror-reopen.md) / [ADR-082](ADR-082-b1-qemu-type-nmi.md) (2026-09-19) — still blocked; park Verified.
- B2 spike: [ADR-084](ADR-084-b2-free-runner-serror.md) (2026-09-25) — free hosted runners have no arm64 KVM/HVF/WHPX; B2 Planned / blocked.
- FAT readdir and other product cuts stay out of this PR.
