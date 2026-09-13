# ADR-042 — Isolation leftover honesty wrap (start-stay + status)

- Status: Accepted (serial / tests prove `_start` stays while live image + heap are torn; umbrella isolation stays Planned)
- Date: 2026-09-13

## Context

Track A / A5 closed a deep isolation ladder: identity `.data` ([ADR-037](ADR-037-identity-data-tear.md)), heap ([ADR-038](ADR-038-identity-heap-tear.md)), EL0 trampoline without `TLBI VMALLE1` ([ADR-039](ADR-039-el0-entry-without-vmalle1.md)), lower-EL IRQ while standing ([ADR-040](ADR-040-lower-el-irq-standing.md)), and IRQ-unmasked default standing/task `ERET` ([ADR-041](ADR-041-irq-unmasked-default-eret.md)). Former stay markers (`ident: data-stay` / `ident: heap-stay`) were superseded by those tears.

What remains Planned must stay **fail-closed** in the ledger and threat model: PAN enable on `-cpu cortex-a57`, lower-EL FIQ/SError, full identity teardown including `_start`, and the umbrella “EL0 isolated” row. QEMU `-kernel` still needs `_start` at `0x4008_0000` — do **not** yank it. This ADR is an honesty wrap of those leftovers, not a claim that isolation closed.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Print `ident: start-stay` (and `ident: ram-stay` for leftover identity frames after the heap) + `#[test_case]` that `0x4008_0000` stays mapped while live `.text` / `.rodata` / `.data` / heap are torn. Document Verified ladder ADR-037…041 vs still-Planned leftovers in this ADR; bump threat-model; keep P-SEC-3l Planned. | Honest wrap. Do not switch `-cpu`. Do not claim “EL0 isolated.” |
| **H2 (rejected)** | Unmap `_start` / claim “kernel moved” / claim umbrella isolation because the ladder exists. | Would break `-kernel` and round up. |
| **H3 (rejected)** | Switch default `-cpu` to enable PAN and call isolation done. | Forbidden by ADR-026 / roadmap. |
| **H4 (deferred)** | Take lower-EL FIQ/SError while standing. | Not cheap on this wrap; leave Planned. |

## Decision

1. **Start-stay marker.** After the ADR-038 heap tear proves live image ranges are unmapped, assert identity `_start` (`KERNEL_TEXT` / `0x4008_0000`) is still mapped+executable and print `ident: start-stay lo=… hi=…`.
2. **Remaining-identity-RAM note.** Print `ident: ram-stay lo=… hi=…` for the still-mapped identity frame bump after the torn heap (`heap_pa_end`…`pool_end`).
3. **Fail-closed probe.** `scripts/qemu-smoke.sh` greps `ident: start-stay` and `ident: ram-stay`. `#[test_case]` `identity_boot_stub_stays_while_live_torn`.
4. **Status table (this wrap).**

| Item | Status | Notes |
| --- | --- | --- |
| ADR-037 identity `.data`/stacks tear | **Verified** | `ident: data*` |
| ADR-038 identity heap tear | **Verified** | `ident: heap*` |
| ADR-039 EL0 entry without `VMALLE1` | **Verified** | `el0: no-vmalle1` |
| ADR-040 lower-EL IRQ while standing | **Verified** | `el0: irq` |
| ADR-041 IRQ-unmasked default ERET | **Verified** | `el0: irq-default` |
| `_start` stay at `0x4008_0000` | **Verified (honesty)** | `ident: start-stay`; not a teardown |
| Remaining identity RAM after heap | **Verified (honesty)** | `ident: ram-stay`; tear still Planned |
| PAN enable on `-cpu cortex-a57` | **Planned** | `pan: absent` (ADR-026). Do not switch `-cpu`. |
| Lower-EL FIQ/SError while standing | **Superseded by ADR-043** | FIQ Verified / SError park honesty on [ADR-043](ADR-043-lower-el-fiq-serror.md). |
| Full identity teardown including `_start` | **Planned** | Do not yank `_start` for QEMU `-kernel`. |
| Umbrella “EL0 isolated” (P-SEC-3l) | **Planned** | Ladder miles are not this row. |

5. **Docs.** Update [el0.md](../framework/el0.md), [security.md](../framework/security.md) (threat-model **v1.22**), honesty ledger, roadmap P-SEC-3l, NFR-10 text in place. Do not mint NFR-15+.
6. **Keep `_start` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN or “EL0 isolated.”

## Honesty

Say “`_start` / boot-stub page stays mapped while live `.text`/`.rodata`/`.data`/heap are torn” only when the serial / tests pass. Do **not** say:

- “EL0 isolated”
- the kernel has moved to the high half
- identity mappings were fully torn down
- PAN enable / lower-EL FIQ/SError are handled
- “secure OS” / “hardened”

## Consequences

- `src/teardown.rs` prints the stay markers; smoke and the new `#[test_case]` ratchet them.
- [ADR-013](ADR-013-el0-isolation-direction.md) remains the isolation direction; P-SEC-3l stays Planned.
- [ADR-043](ADR-043-lower-el-fiq-serror.md) takes FIQ and documents SError park honesty. A later ADR may take SError safely, enable PAN on a different CPU story, or redesign boot so `_start` can leave identity.
