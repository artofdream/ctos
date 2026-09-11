# ADR-016 — TTBR1 kernel-private page (first cut)

- Status: Accepted (first cut; identity teardown Planned)
- Date: 2026-09-11

## Context

[ADR-013](ADR-013-el0-isolation-direction.md) left TTBR1 / higher-half as a missing isolation mile. The kernel still runs from the QEMU `virt` identity map at `0x4008_0000` (TTBR0, T0SZ=25). `TCR.EPD1` was set, so TTBR1 walks were disabled. EL0 and EL1 share the same translation regime: a high mapping is not automatically invisible to EL0 — access is an AP / UXN question.

A full higher-half relocate (kernel fetch, `VBAR_EL1`, stacks, UART MMIO, then tearing down the identity image) is too large for one honest PR. This ADR is the **smallest Verified cut**: enable TTBR1 and prove one kernel-private high page.

## Decision

1. **Enable TTBR1 walks.** Clear `TCR.EPD1`. Set T1SZ=25 (same 39-bit window as T0SZ) with WBWA / inner-shareable attrs on TTBR1. ASID still comes from TTBR0 (`TCR.A1 = 0`).
2. **High-half base.** The TTBR1 region is `0xFFFF_FF80_0000_0000`…`0xFFFF_FFFF_FFFF_FFFF`. One private page lives at `TTBR1_PRIV = 0xFFFF_FF80_0000_0000` (`src/paging.rs`).
3. **EL1-only leaf.** The page is Normal, RW, PXN, UXN, AP[2:1]=00 (EL1 RW, EL0 no data access). Backing store is a frame-pool page, also identity-mapped for EL1. User TTBR0 does not describe this VA.
4. **Fail-closed probe.** EL1 store/load via the high VA prints `ttbr1: el1`. An EL0 `LDR` of `TTBR1_PRIV` is a lower-EL permission (or translation) DABORT (`ttbr1: no el0`). Serial `ttbr1: ok`. `scripts/qemu-smoke.sh` greps those strings and rejects `ttbr1: probe missed` / `ttbr1: leaked`. `#[test_case]` covers the same path.
5. **Identity map stays.** `_start`, vectors, linker stacks, PL011 `0x0900_0000`, and the multi-L3 / layout ratchets from #21 keep using TTBR0 identity VAs. **Full higher-half + identity teardown is Planned.**
6. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+. Do not claim “the kernel runs in the high half” or “EL0 isolated.”

## Honesty

Say “TTBR1 maps a kernel-private page EL0 cannot access” only when the serial / tests pass. Do **not** say:

- the kernel has moved to a high VA
- identity mappings were torn down
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::init` programs `TTBR1_EL1` and leaves `EPD1` clear. `src/ttbr1.rs` owns the probe.
- Standing EL0 (ADR-013) is a separate mile. Umbrella isolation stays Planned while PAN is unclaimed.
- [ADR-017](ADR-017-ttbr1-high-el1-exec.md) aliases identity RAM in TTBR1 and fetches a real EL1 path (plus high `VBAR_EL1`). [ADR-018](ADR-018-identity-teardown-first-cut.md) splits those tables and unmaps one identity text page. [ADR-019](ADR-019-identity-text-range-tear.md) unmaps a 16 KiB dedicated text range after a high-VA jump. Full identity teardown is still Planned.
