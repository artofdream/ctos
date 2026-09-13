# ADR-049 — Leftover identity RAM tear after the heap

- Status: Accepted (leftover identity frames after the heap unmapped; high twins stay; `_start` stays) when the serial / tests pass
- Date: 2026-09-13

## Context

[ADR-038](ADR-038-identity-heap-tear.md) unmapped identity heap while `GlobalAlloc` returned TTBR1 VAs. [ADR-042](ADR-042-isolation-leftover-wrap.md) printed honesty stay markers: `ident: start-stay` for `_start` at `0x4008_0000` and `ident: ram-stay` for leftover identity frames in `[heap_pa_end, pool_end)`. [ADR-047](ADR-047-isolation-leftovers-decisions.md) decided **never yank `_start`** while QEMU `-kernel` needs that address, and left leftover frame RAM as an **optional** Planned mile (not umbrella-required).

This ADR takes that optional mile: unmap leftover identity frame RAM after the torn heap, keep high twins, keep `_start` mapped+executable, and fail-closed on serial / tests / smoke. It does **not** claim “EL0 isolated,” “kernel moved,” PAN enable, or taken SError Verified. Umbrella P-SEC-3l stays Planned/non-claim.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Unmap identity frames in `[heap_pa_end, pool_end)`. Keep boot-stub pages needed for `-kernel`. Print `ident: ram` / `ident: ram-fault` / `ident: ram-high`. Access newly allocated frames via the TTBR1 twin (`frame_cpu_va`). | Smallest honest optional tear. |
| **H2 (rejected)** | Yank `_start` / claim full higher-half / claim “EL0 isolated.” | Forbidden by ADR-047 / roadmap. |
| **H3 (rejected)** | Leave `ident: ram-stay` forever and mark the optional mile Verified without a tear. | Would invent closure. |

## Decision

1. **Unmap leftover identity RAM.** After `tear_identity_heap`, `tear_identity_ram` unmaps `[heap_pa_end, pool_end)` from kernel TTBR0. Full 2 MiB L2 blocks are cleared as one slot; partial edge blocks are punched page-by-page. User TTBR0 already omits that range. High twins stay (`L2_HIGH_RAM`). One boot-time `TLBI VMALLE1` after the bulk unmap.
2. **Plant + probe.** Before unmap, plant a magic word via the high twin. Serial `ident: ram lo=… hi=… pages=N` with `N >= 1`. EL1 `LDR` of a torn identity leftover VA is a current-EL translation DABORT (`ident: ram-fault`). EL1 `LDR` of the high twin still returns the planted magic (`ident: ram-high`).
3. **Keep `_start`.** `ident: start-stay` must still pass. Do not change default `-cpu`. Do not claim PAN / “EL0 isolated” / “kernel moved.”
4. **Frame CPU VA.** Call sites that used a frame PA as an identity VA (`paging::observe_probe`, `map_unmap_roundtrip`, `loader`, `asid`, `ttbr1` private-page plant) switch to `paging::frame_cpu_va` (TTBR1 twin after the high split).
5. **Fail-closed smoke.** `scripts/qemu-smoke.sh` requires `ident: ram lo=` / `ident: ram-fault` / `ident: ram-high` and **rejects** `ident: ram-stay`. Keep requiring `ident: start-stay`.
6. **Still Planned / decided.** Umbrella “EL0 isolated” (P-SEC-3l) stays Planned/non-claim. PAN enable non-goal on default a57. Taken SError deferred/non-goal on this smoke machine. Never yank `_start`.
7. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.27**. Do not mint NFR-15+.

## Honesty

Say “leftover identity frame RAM after the heap was unmapped while the high twin stayed and `_start` stayed mapped” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down (`_start` stays)
- “secure OS” / “hardened” / “EL0 isolated”
- PAN enable / taken SError Verified

## Consequences

- `paging::tear_identity_ram` / `frame_cpu_va` / teardown probes own the cut. [ADR-042](ADR-042-isolation-leftover-wrap.md) `ident: ram-stay` is superseded as a stay marker; `ident: start-stay` remains.
- [ADR-047](ADR-047-isolation-leftovers-decisions.md) leftover-RAM row becomes Verified (optional mile taken); umbrella stays Planned/non-claim.
