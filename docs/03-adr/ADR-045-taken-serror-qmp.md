# ADR-045 — Taken lower-EL SError via QMP/`nmi` (honest partial)

- Status: Accepted as **implementation attempt** — guest A-clear standing path is wired; host QMP `inject-nmi` on QEMU 10 `virt` + `-cpu cortex-a57` does **not** deliver SError (`machine does not provide NMIs`). Taken lower-EL SError while standing stays **Planned**. `el0: serror-park` remains the Verified honesty marker ([ADR-043](ADR-043-lower-el-fiq-serror.md) / [ADR-044](ADR-044-taken-serror-research.md)).
- Date: 2026-09-13

## Context

[ADR-044](ADR-044-taken-serror-research.md) recommended **H1**: host QMP/`inject-nmi` while standing with `PSTATE.A` clear, expecting virt → `ARM_CPU_SERROR`. That recommendation tracked a 2020 qemu-devel series; **upstream QEMU 10 does not implement that path** on this smoke machine:

- `inject-nmi` / HMP `nmi` → `machine does not provide NMIs` (no `TYPE_NMI` on virt for cortex-a57 + default GICv2).
- Modern virt “NMI” support is **FEAT_NMI** (GICv3 + a CPU with `aa64_nmi`), not async SError, and requires switching `-cpu` / GIC — forbidden here.
- TCG has virtual SError (`HCR_EL2.VSE` / `EXCP_VSERR`) for EL2→EL1, not a host inject into our EL1/EL0 standing guest.
- Guest-only RAS inject remains rejected for Verified ([ADR-044](ADR-044-taken-serror-research.md) H2).

Do **not** switch default `-cpu cortex-a57`. Do **not** claim “EL0 isolated.” Keep `_start` at `0x4008_0000`. Do **not** unmask A on default standing/task `ERET` (SPSR `0x340` keeps A set).

## Decision

1. **Guest plumbing (shipped).** `eret_to_el0_serror` uses SPSR `0x2c0` (D+I+F set, **A clear**). Hello arms `EXPECT_EL0_SERROR`, prints UART cue `el0: serror-arm`, enters standing dual-SVC with a **bounded spin** (not `WFI` — I/F stay masked, so a failed inject must not hang). Handler `serror_lower_el` prints `el0: serror` only when EXPECT is armed, then `stay_at_el0`.
2. **Host smoke (attempted).** `scripts/qemu-serial-inject.py` opens `-qmp unix:…,server,nowait` and, after `el0: serror-arm`, issues `{"execute":"inject-nmi"}`, logging the result. On this QEMU it errors; guest prints `el0: serror-park`. `qemu-smoke.sh` still **requires** `el0: serror-park` and must **not** require `el0: serror`.
3. **No `#[test_case]`** for the taken path — it needs a host inject. Smoke-only honesty.
4. **Taken path stays Planned.** Do not mark Verified. Park marker stays Verified.
5. **Still Planned (unchanged honesty):** PAN enable on `-cpu cortex-a57` ([ADR-026](ADR-026-pan-capability.md)); yank `_start` / leftover identity RAM ([ADR-042](ADR-042-isolation-leftover-wrap.md)); umbrella “EL0 isolated.”
6. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.25**. Do not mint NFR-15+.

## Honesty

Say “guest A-clear standing SError path is wired; QMP `inject-nmi` does not take SError on virt/cortex-a57.” Do **not** say:

- taken lower-EL SError is Verified on this guest
- “EL0 isolated” / PAN / “secure OS”
- default standing `ERET` unmasks SError
- ADR-044 H1 landed as a working inject on QEMU 10
- guest-only RAS inject works on cortex-a57 TCG

## Consequences

- `src/exception.rs` / `src/el0.rs` own the A-clear probe + EXPECT. `scripts/qemu-serial-inject.py` owns the QMP attempt. `el0: serror-park` stays the fail-closed serial when inject does not land.
- A later ADR may find a **different** honest inject (new QEMU feature, accepted non-goal, or a constrained CPU story with sponsor approval). That work is not this cut.
- Docker/(evo-x2) re-probe is optional; agent-box smoke must show park + QMP error log, not a fake `el0: serror`.
