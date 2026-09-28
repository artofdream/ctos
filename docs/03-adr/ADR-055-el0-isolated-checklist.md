# ADR-055 — Umbrella “EL0 isolated” non-claim checklist

- Status: Accepted (docs). **Since M6 (2026-09-28) every §B row is Met and the umbrella is Verified only as the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) sentence.** *Original status text (2026-09-14):* Umbrella “EL0 isolated” stays **Planned / non-claim** and **cannot become Verified** under current constraints. This ADR tightens the non-claim with an evidence checklist; it does **not** invent Verified.
- Date: 2026-09-14
- **Follow-on (2026-09-28, M6):** the sponsor accepted the ADR-091 revision-3 sentence and scope in writing (B3, 2026-09-28 16:41 CEST; sponsor wrote it in the DSO agent chat, DSO relayed it to ctos). Row 3 is **Met**. [ADR-091](ADR-091-el0-isolated-accept-draft.md) is **Accepted**. The umbrella row flips to **Verified**, scoped to that sentence and its limits.
- **Follow-on (2026-09-27, ADR-092):** sponsor option (b) on ADR-091. G1 (EL0-reachability walk) and G2 (EL0 write fault) closed as §A miles below; new gap G4 (syscall copy helpers skip the EL0 permission check) recorded. Row 3 still **not Met** (ADR-091 revision 2 is Draft).
- **Follow-on (2026-09-26, M5):** the umbrella sentence and scope are **drafted** in [ADR-091](ADR-091-el0-isolated-accept-draft.md) (Draft / pending sponsor accept, D4). Row 3 stays **not Met** until the sponsor has read the draft and accepted it in writing. Nothing is flipped.
- **Follow-on (2026-09-26, M4):** sponsor D1 accepted. The taken-SError evidence class for the umbrella is the B1 pin job (now **unconditional** on every push/PR) plus the B2-P one-off KVM run ([ADR-090](ADR-090-serror-evidence-class.md)). Row 1 is reworded and Met on that class. Stock QEMU still parks. The umbrella still needs the sponsor's written accept (M5 draft → ADR-091).
- **Audit (2026-09-26):** [ADR-086](ADR-086-el0-isolated-checklist-audit.md) re-probed every row this session. All §A miles are Verified. All §B rows are still **not** Met: taken SError is Verified only on opt-in paths (B1 pin / B2-P one-off). The identity inventory still leaves five identity ranges (`_start` is one of them, by decision). No umbrella accept exists. §A marker `el0: no data` is stale wording: the code uses `el0: no kernel read`.
- Locks [ADR-047](ADR-047-isolation-leftovers-decisions.md) decision #4. Optional ledger clarity: “non-claim until checklist.”

## Context

Many Track A / A5 miles are **Verified** (standing EL0, IRQ/FIQ while standing, identity tears, PAN **ID-field**, SError **park**, start-stay, leftover RAM tear, …). Rounding those miles up to “EL0 isolated” is forbidden. Sponsor / DSO asked to **secure** the umbrella gap before Guest Linux / containers / immutable-OS marketing: publish a checklist of Verified miles vs umbrella requirements still unmet, given:

- PAN enable was locked on default a57 ([ADR-054](ADR-054-pan-enable-lock.md)); reopen + enable on cortex-a76 ([ADR-079](ADR-079-pan-cpu-reopen.md) / [ADR-080](ADR-080-pan-enable-fault.md)) — still not the umbrella
- `_start` stays / never yank ([ADR-042](ADR-042-isolation-leftover-wrap.md) / [ADR-047](ADR-047-isolation-leftovers-decisions.md))
- Taken lower-EL SError hard-stopped / deferred ([ADR-053](ADR-053-taken-serror-hard-stop.md); reopen re-probe [ADR-081](ADR-081-taken-serror-reopen.md) / [ADR-082](ADR-082-b1-qemu-type-nmi.md))

CloudAgent HELD; docs-only.

## Decision

1. *(Superseded by M6, 2026-09-28: all §B rows Met; the umbrella is Verified as the ADR-091 sentence only.)* **Umbrella stays Planned / non-claim.** Ledger wording may say **“non-claim until checklist”** (this ADR). Do not flip to Verified.
2. **Checklist is the gate.** “EL0 isolated” may be reconsidered only when **every** Unmet row below is Met under its own ADR + probes — not by summing existing miles.
3. **Specific Verified miles are not the umbrella.** Cite them as themselves.

## Evidence checklist

### A. Verified miles (not sufficient for the umbrella)

| Mile | Evidence (serial / test) | ADR |
| --- | --- | --- |
| EL0 first mile + NX kernel data | `el0: ok` / `el0: nx kernel` | ADR-013 |
| User TTBR0 read mile | `el0: no kernel read` (wording fixed 2026-09-26, [ADR-087](ADR-087-identity-inventory-ratchet.md); earlier text said `el0: no data`, which no code prints) | ADR-013 |
| Standing EL0 + standing task | `el0: standing` / `el0: task-ok` | ADR-013 / ADR-024 |
| ASID isolation | `asid: ok` | ADR-013 |
| TTBR1 private page + high EL1 fetch | `ttbr1: ok` / `ttbr1: el1 exec` | ADR-016 / ADR-017 |
| Identity tears (text/rodata/data/heap/ram) | `ident: *` / `ident: ok` | ADR-018…020 / 025 / 037 / 038 / 049 |
| EL0 entry without `VMALLE1` | `el0: no-vmalle1` | ADR-039 |
| Lower-EL IRQ while standing + default I clear | `el0: irq` / `el0: irq-default` | ADR-040 / ADR-041 |
| Lower-EL FIQ while standing | `el0: fiq` | ADR-043 |
| SError **park** honesty | `el0: serror-park` | ADR-043 / ADR-045 |
| PAN **ID-field** on default probe CPU | `pan: present` on cortex-a76 (ADR-079); historical a57 `pan: absent` | ADR-026 / ADR-079 |
| PAN **enable** + EL1-vs-EL0 fault | `pan: enabled` / `pan: el1-fault` | ADR-080 |
| EL0 **store** to kernel data (TTBR1) / EL0-RO page → permission fault (2026-09-27) | `el0: write-fault kernel … ec=0x24 wnr=1 dfsc=perm-l3` / `el0: write-fault user-ro …` / `el0: write-ok kernel,user-ro` | [ADR-092](ADR-092-el0-reach-walk-write-fault.md) |
| Fail-closed EL0-reachability walk (k/u/ASID-B TTBR0 + TTBR1; allowlist = five user slots) (2026-09-27) | `el0-reach: live k=3 u=3 a=0 h=0 pages=6 leaks=0` / `el0-reach: steady … pages=0 leaks=0` / `el0-reach: neg … caught` / `el0-reach: ok allow=…` | [ADR-092](ADR-092-el0-reach-walk-write-fault.md) |
| `_start` stay honesty | `ident: start-stay` | ADR-042 / ADR-047 |

### B. Umbrella requirements (were **Unmet**; all **Met** since M6, 2026-09-28)

| Requirement | Current status | Why unmet | Reopen / lock ADR |
| --- | --- | --- | --- |
| Taken lower-EL SError while standing, **on the D1 evidence class** (reworded 2026-09-26 per sponsor D1, [ADR-090](ADR-090-serror-evidence-class.md)): B1 `qemu-nmi-pin` green, unconditional on every push/PR, plus B2-P ADR-085 run 4 | **Met (D1 evidence class; not the umbrella)**. B1 bare `el0: serror` fail-closed in CI; B2-P `el0: serror` + `b2: taken` under Graviton3 KVM (one-off, at `159b178`). Stock virt TCG itself: still **deferred / non-goal (hard-stopped)**, `el0: serror-park` | Stock QEMU has no honest inject on virt TCG (`machine does not provide NMIs`) — re-probed a76 + GICv3/virt-on/max 2026-09-19. B1 opt-in pin Verified ([ADR-083](ADR-083-b1-qemu-nmi-pin.md)); B2 free-runner spike blocked 2026-09-25 ([ADR-084](ADR-084-b2-free-runner-serror.md)) | [ADR-053](ADR-053-taken-serror-hard-stop.md) / [ADR-081](ADR-081-taken-serror-reopen.md) / [ADR-082](ADR-082-b1-qemu-type-nmi.md) / [ADR-083](ADR-083-b1-qemu-nmi-pin.md) / [ADR-084](ADR-084-b2-free-runner-serror.md) |
| Full identity teardown **except the documented `_start` stub page** (reworded 2026-09-26 per sponsor D2, [ADR-089](ADR-089-start-stub-identity-exception.md); was “including yank `_start`”) | **Met (this session; not the umbrella)**. The fail-closed inventory reports only the stub page: `ident: inv k=1 u=1 a=1 leaks=0` / `ident: inv-ok allow=stub` | M1 [ADR-087](ADR-087-identity-inventory-ratchet.md) + M2 [ADR-088](ADR-088-mmio-high-alias.md). `_start` never yanked (`ident: start-stay`) | ADR-042 / ADR-047 (amended) / [ADR-089](ADR-089-start-stub-identity-exception.md) |
| Written sponsor accept that the umbrella sentence is in scope | **Met (2026-09-28 16:41 CEST, B3)**. Verbatim: “I accept the ADR-091 revision-3 sentence and scope as written. After M6, have ctos propose a short costed list of next milestones.” Relay path: sponsor wrote it in the DSO agent chat, DSO relayed it to ctos. | Was absent until 2026-09-28 (draft since 2026-09-26, D4; revisions 2 and 3 after G1/G2/G4). | [ADR-091](ADR-091-el0-isolated-accept-draft.md) (Accepted) → M6 flip |

§B is cleared (M6, 2026-09-28): the umbrella row is **Verified**, scoped to the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) sentence and its limits. *Original text:* Until §B is cleared under honest probes (and `_start` policy is redesigned with sponsor scope if ever), the umbrella row stays **non-claim**.

## Honesty

Since M6, say “EL0 isolated” only as the accepted [ADR-091](ADR-091-el0-isolated-accept-draft.md) sentence, with its limits. Many isolation miles are Verified; one mile alone is not the umbrella. *(Before M6 this read: “umbrella EL0 isolation is Planned / non-claim until the ADR-055 checklist; many isolation miles are Verified but that is not ‘EL0 isolated.’”)* Do **not** say:

- “EL0 isolated” wider than the ADR-091 sentence / “secure OS” / “hardened isolation complete”
- “EL0 isolated” from PAN alone / taken SError Verified / `_start` yanked
- Guest Linux / containers / immutable-OS marketing as if isolation were closed

## Consequences

- Honesty ledger umbrella row cites this checklist; optional label **“non-claim until checklist.”**
- [el0.md](../framework/el0.md), roadmap P-SEC-3l, [security.md](../framework/security.md), [limits.md](../overview/limits.md) point here.
- Sponsor-facing one-pager (locks + reopen gates table): [ADR-060](ADR-060-isolation-leftovers-closure-checklist.md).
- No `src/` change. No Verified invent. Threat-model patch bump only.
