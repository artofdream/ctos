# ADR-080 — PAN enable + EL1-vs-EL0 access fault (ADR-054 reopen gate 3–4)

- Status: Accepted (enable + fault Verified on default `-cpu cortex-a76`). Meets [ADR-054](ADR-054-pan-enable-lock.md) reopen gate items **3–4**: enable PSTATE.PAN / `MSR PAN` and prove an EL1 load of an EL0-accessible page faults (fail-closed); honesty ledger + smoke updated. Does **not** claim “EL0 isolated.” Taken SError reopen is **[ADR-081](ADR-081-taken-serror-reopen.md)** (Accepted — still blocked).
- Date: 2026-09-19
- Tracks [#125](https://github.com/artofdream/ctos/issues/125) mile (2). Builds on [ADR-079](ADR-079-pan-cpu-reopen.md) (gate items 1–2). CloudAgent **HELD** — agent-box / EVO-X2 / GHA only.

## Context

[ADR-054](ADR-054-pan-enable-lock.md) locked PAN **enable** until a follow-up ADR after FEAT_PAN CPU reopen. [ADR-079](ADR-079-pan-cpu-reopen.md) met gate items 1–2 (`-cpu cortex-a76`, `pan: present`). Gate item 3 required this ADR: enable PSTATE.PAN and prove EL1-vs-EL0 fault. Gate item 4: ledger + smoke for enable; keep a57 `pan: absent` historical.

[ADR-013](ADR-013-el0-isolation-direction.md) already named the enable mile: with PSTATE.PAN set, an EL1 data access to a page that is unprivileged-accessible must take a permission fault. Umbrella “EL0 isolated” stays non-claim ([ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md)). Never yank `_start`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | After ID-field `pan: present`, `MSR PAN, #1`; map an EL0-RO page; EL1 `LDR` must permission-fault with SPSR.PAN set; print `pan: enabled` / `pan: el1-fault`; leave PAN set; syscall uaccess clears PAN around copies. | Fail-closed if fault missing or SPSR.PAN clear. Preserves net/FAT/samples. |
| **H2 (rejected)** | Enable for the probe only, then clear PAN for the rest of boot. | Weaker than “enable”; uaccess wrapping is small and honest. |
| **H3 (rejected)** | Claim “EL0 isolated” from this mile alone. | ADR-055 §B still unmet (taken SError, `_start` stay, umbrella sponsor accept). |
| **H4 (rejected)** | Invent Verified without a caught fault / silent `-cpu`. | Honesty fail. |

## Decision

1. **Enable.** `src/pan.rs` `observe_enable` (after `frame::init`) executes `MSR PAN, #1` on FEAT_PAN CPUs (`ID_AA64MMFR1_EL1.PAN != 0`). Serial `pan: enabled`.
2. **EL1-vs-EL0 fault.** Map an EL0-accessible RO page (`map_el0_ro`); with PAN clear, EL1 preload must succeed; with PAN set, EL1 `LDR` must take a current-EL permission DABORT. Handler prints `pan: el1-fault` and requires **SPSR.PAN (bit 22)** set at the fault (fail-closed otherwise). Note: under QEMU 10.0.13 TCG, `MRS PAN` may read 0 even while SPSR.PAN and the permission check prove enable — SPSR is the probe of record here.
3. **Leave PAN set.** After a successful probe, PSTATE.PAN stays 1. `pan::with_user_access` clears/restores around syscall `copy_user` / `copy_to_user` so intentional EL0-accessible copies keep working.
4. **Serial + smoke.** Keep `pan: id=` / `pan: present` (ADR-079). Require `pan: enabled` / `pan: el1-fault`. Reject `pan: enable missed` / unexpected `pan: absent`. Historical a57 `pan: absent` rows remain historical.
5. **Target feature.** `aarch64-ctos.json` adds `+pan` so the assembler accepts the `pan` PSTATE name (still `-cpu cortex-a76` in scripts).
6. **Still non-claim.** Umbrella “EL0 isolated” unchanged. Taken SError → [ADR-081](ADR-081-taken-serror-reopen.md) (blocked). Never yank `_start`. No Guest Linux / containers marketing.

## Honesty

Say: “on `-cpu cortex-a76` (ADR-079), PSTATE.PAN is enabled (ADR-080); EL1 load of an EL0-accessible page faults (`pan: el1-fault`); umbrella EL0 isolation stays non-claim.” Do **not** say:

- “EL0 isolated” / “secure OS”
- taken lower-EL SError is Verified
- PAN worked on cortex-a57 without a CPU change
- Guest Linux / containers / immutable-OS

## Consequences

- `src/pan.rs`, `src/exception.rs` (arm + SPSR.PAN check), `src/syscall.rs` (uaccess), `src/main.rs`, `scripts/qemu-smoke.sh`, `aarch64-ctos.json`.
- Docs: this ADR; [ADR-054](ADR-054-pan-enable-lock.md) / [ADR-055](ADR-055-el0-isolated-checklist.md) / [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md) / [ADR-026](ADR-026-pan-capability.md) / [ADR-079](ADR-079-pan-cpu-reopen.md) cross-links; honesty ledger; roadmap P-SEC-3k; threat-model **v1.55**; SUMMARY; el0 / limits / security light touch.
- Preserve all existing net/FAT/sample smoke greps. CloudAgent HELD. Do not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
