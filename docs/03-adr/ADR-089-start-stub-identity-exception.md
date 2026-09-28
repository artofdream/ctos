# ADR-089 — `_start` stub page is the one documented identity exception (M3, sponsor D2)

- Status: Accepted (docs). Records sponsor decision **D2** (DSO relay 2026-09-26 ~14:35 CEST): keep the `_start` boot-stub page (**I3**) as the **one** documented identity exception, never yank `_start`, and reword ADR-047 §3 plus the checklist's identity-teardown row to match. No `src/` change. The umbrella “EL0 isolated” stays **Planned / non-claim**.
- Date: 2026-09-26
- Base: stacked on PR [#140](https://github.com/artofdream/ctos/pull/140) (ADR-088, M2).

## Context

ADR-055 §B row 2 read “Full identity teardown including yank `_start`”, while [ADR-047](ADR-047-isolation-leftovers-decisions.md) §3 decided “never yank `_start`”. [ADR-086](ADR-086-el0-isolated-checklist-audit.md) flagged the row as unmeetable as worded and asked the sponsor (D2). After [ADR-087](ADR-087-identity-inventory-ratchet.md) (M1) and [ADR-088](ADR-088-mmio-high-alias.md) (M2), a fail-closed walk shows that the stub page is the **only** identity mapping left in any TTBR0.

## Decision

1. **The exception.** Exactly one identity (VA = PA) mapping may remain: the page `[0x4008_0000, 0x4008_1000)` that holds `_start` and the identity copy of `exception_vectors` (VBAR is high). It must be EL1 read-only + executable, never EL0-accessible, and present in kernel, user and ASID-B TTBR0 as it is today. `paging::INV_ALLOW` encodes exactly this page. Any other identity leaf fails `scripts/qemu-smoke.sh` (`ident: inv-leak`).
2. **Never yank or move `_start`.** QEMU `-kernel` enters at `0x4008_0000`; the image stays there. (ADR-086 D2 option (b), unmapping the stub's identity mapping after boot, was **not** chosen.)
3. **Reworded requirement** (ADR-055 §B row 2, ADR-060 “Yank `_start`” / umbrella rows): *“Full identity teardown except the documented `_start` stub page (ADR-089): the fail-closed TTBR0 identity inventory reports only that page (`ident: inv k=1 u=1 a=1 leaks=0` + `ident: inv-ok allow=stub`) on the default smoke.”*
4. **ADR-047 §3 amended in place** (history kept): “never yank `_start`” stands; leftover identity RAM is no longer “optional”, since ADR-049 + ADR-087 tore it; the stub page is the one documented exception.
5. **Row status after this ADR:** reworded row 2 = **Met on evidence from this session**. That evidence is the ADR-088 box smoke 2026-09-26 ~14:43 CEST, with CI on #140 recorded in the ledger once green. That is **not** the umbrella: rows 1 (taken SError → M4 / D1) and 3 (sponsor accept → M5 draft, then the sponsor's own accept) remain.

## Future narrowing (not in scope, not decided)

The stub page is in **user** TTBR0 only because `fill_user_map` maps `[KERNEL_TEXT, data)` as EL1 text. VBAR is high, so EL0 exceptions do not fetch it through TTBR0. Removing it from user TTBR0 alone would leave kernel TTBR0 and `_start` untouched. That would need a sponsor call; this ADR does not act on it.

## Honesty

Say: “the only identity mapping left is the documented `_start` stub page (EL1 RO+X), ratcheted fail-closed.” Do **not** say “identity fully torn down”, “`_start` yanked”, “kernel moved”, or “EL0 isolated”.

## Consequences

- ADR-047 §3, ADR-055 §B, ADR-060 and ADR-042 honesty wording point here. Ledger, roadmap P-SEC-3l, threat model **v1.64**, [el0.md](../framework/el0.md), SUMMARY, MOC.
- Next: M4 (D1 evidence set), M5 (draft accept). Stop before M6.
