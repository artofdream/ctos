# ADR-044 — Taken lower-EL SError research (no Verified taken path yet)

- Status: Accepted as **research** — taken lower-EL SError while standing stays **Planned**; `el0: serror-park` remains the Verified honesty marker ([ADR-043](ADR-043-lower-el-fiq-serror.md))
- Date: 2026-09-13

## Context

[ADR-043](ADR-043-lower-el-fiq-serror.md) wired a defensive standing-aware `serror_lower_el` stub and prints `el0: serror-park` because this cut had **no safe, deterministic in-guest inject** on QEMU `virt` + TCG + `-cpu cortex-a57`. Sponsor asked for a research mile (Grok/docs) before claiming a taken path.

Default standing/task SPSR still **masks** SError (`PSTATE.A` set; ADR-041 SPSR `0x340` is D+A+F with I clear). A taken probe must clear **A** (and keep the guest standing) the same way the FIQ probe clears **F**.

Do **not** switch default `-cpu`. Do **not** claim “EL0 isolated.” Keep `_start` at `0x4008_0000`.

## Options surveyed

| ID | Option | Feasibility on ctos smoke | Honesty |
| --- | --- | --- | --- |
| **H1 (recommended next mile)** | Host-driven inject via QEMU HMP/QMP **`nmi`**, which on `virt` raises the CPU **SError** line (`ARM_CPU_SERROR` / TCG `CPU_INTERRUPT_SERROR`). Run smoke with a QMP socket; while standing EL0 (A clear + `WFI`), issue `{"execute":"inject-nmi"}` / HMP `nmi`. Serial `el0: serror` (taken) + existing park marker only when not probing. | **Feasible** with scripted QMP; needs smoke harness change (`-qmp` / unix socket), timing race around standing window, and fail-closed if QMP unavailable. Not an in-guest SVC. | Honest: host inject is how virt simulates NMI→SError. |
| **H2 (rejected for Verified)** | Guest-only RAS / imprecise abort / poisoned load to force async SError. | Unreliable on `-cpu cortex-a57` TCG virt without RAS story; easy to confuse with sync aborts. | Would round up. |
| **H3 (rejected)** | Treat a sync abort / BRK / FIQ as “SError.” | Easy, wrong EC. | Forbidden. |
| **H4 (status quo)** | Keep `el0: serror-park` Verified; taken path Planned until H1 ships. | Current main (`65cf515`). | Correct until H1 probes. |
| **H5 (out of scope)** | KVM `KVM_SET_VCPU_EVENTS` `serror_pending` or plugin `arm_cpu_inject_exception`. | ctos smoke is TCG user-mode style `-kernel` on virt, not KVM guest tooling in CI. | Useful notes only. |

### Vector / mask notes

- Lower-EL AArch64 SError vector is `VBAR_EL1 + 0x580` (already wired).
- Taken SError requires **`PSTATE.A` clear** at EL0. Add `eret_to_el0_serror` (e.g. SPSR with A clear, I/F set as needed) for the probe only — do **not** unmask A on default standing `ERET` in the research mile.
- After a taken trip, restore A-masked default so normal smoke stays stable.

### Smoke harness sketch (not implemented here)

1. `qemu-system-aarch64 … -qmp unix:./qmp.sock,server,nowait` (or stdio QMP in a side channel).
2. Hello path: arm `EXPECT_EL0_SERROR`, `eret_to_el0_serror`, payload `SVC #1` / short spin or `WFI`.
3. Host: wait for a UART cue (or fixed delay after `el0: standing`), then QMP `inject-nmi`.
4. Grep `el0: serror` (taken); reject claiming Verified if only park printed.
5. GHA matrix must have a QMP-capable `qemu-system-aarch64` (already true on ubuntu runners once flags are added).

## Decision

1. **This ADR is research only.** No taken-path Verified claim. No `src/` change required to accept it.
2. **Recommended follow-up implementation** is **H1** (QMP/`nmi` while standing with A clear), as a separate ADR (e.g. ADR-045) with probes + qemu-smoke greps.
3. **Keep H4** until that follow-up lands: `el0: serror-park` stays Verified; taken lower-EL SError stays **Planned**.
4. **Still Planned (unchanged honesty):** PAN enable on `-cpu cortex-a57` ([ADR-026](ADR-026-pan-capability.md)); yank `_start` / leftover identity RAM ([ADR-042](ADR-042-isolation-leftover-wrap.md)); umbrella “EL0 isolated.”
5. Threat-model text may note “taken SError research recorded; inject path identified as QMP nmi” without minting NFR-15+.

## Honesty

Say “we know how we would take lower-EL SError on virt” (QMP `nmi` + A clear). Do **not** say:

- taken SError is Verified on this guest
- “EL0 isolated” / PAN / “secure OS”
- default standing `ERET` unmasks SError
- guest-only RAS inject works on cortex-a57 TCG

## Consequences

- [ADR-043](ADR-043-lower-el-fiq-serror.md) remains the FIQ Verified + SError park cut.
- Implementers should open a new ADR for H1, not widen this file into a silent Verified.
- Docker/(evo-x2) smoke does not need re-probe for this docs-only research PR.