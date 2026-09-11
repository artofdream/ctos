# ADR-017 — EL1 fetch from the TTBR1 RAM alias

- Status: Accepted (exec mile; first identity-tear cut is ADR-018)
- Date: 2026-09-11

## Context

[ADR-016](ADR-016-ttbr1-private-page.md) enabled TTBR1 walks and proved one EL1-only *data* page at `TTBR1_PRIV`. The kernel still fetched instructions from the QEMU `virt` identity map at `0x4008_0000`. Full higher-half relocate (every pointer, every `fn` item, then tearing down identity) is still too large for one honest PR: rustc emits link-time identity addresses (`relocation-model: static`), and `-kernel` loads `_start` at `TEXT_OFFSET`.

This ADR is the **largest Verified cut** that still keeps the boot stub: alias identity RAM in TTBR1 and run a real EL1 code path (instruction fetch, not a private store) at high VA.

## Decision

1. **RAM alias.** `L1_HIGH[1]` points at the same `L2_RAM` used by identity TTBR0. High VA is `identity + TTBR1_BASE` (`0x4008_0000` → `0xFFFF_FF80_4008_0000`). Permissions match the identity image (text RO+X, data/stacks/heap RW+NX, guards unmapped).
2. **Shared tables (superseded for the tear mile).** This ADR reused `L2_RAM`. [ADR-018](ADR-018-identity-teardown-first-cut.md) clones RAM tables so one identity text page can be unmapped without dropping the high twin. Full identity teardown is still Planned.
3. **EL1 exec probe.** `ttbr1_high_el1_path` is invoked through its high VA (`BLR` / fn pointer). It captures PC with `ADR`, prints `ttbr1: el1 exec` from that path, and returns the PC. The caller checks `PC` is in the TTBR1 window and near the high entry. UART MMIO stays the absolute identity `0x0900_0000`.
4. **`VBAR_EL1` high.** After the MMU is on, `VBAR_EL1` is programmed to the high alias of `exception_vectors` (`ttbr1: vbar`). Later BRK / IRQ / EL0 sync fetch the table via TTBR1. Identity `exception::init` still installs the link address first (pre-MMU).
5. **Identity boot stub stays.** `_start`, QEMU `-kernel` load, and link-time `fn` items remain at `0x4008_0000`. Most EL1 data access still uses identity VAs. **Do not say the kernel moved.**
6. **Still Planned.** Full identity teardown (unmap all low `.text`/`.data`/heap after a complete high-VA jump); PAN on `-cpu cortex-a57`; umbrella EL0 isolation. [ADR-018](ADR-018-identity-teardown-first-cut.md) is the first cut (one torn text page). User TTBR0 still maps most kernel text so a missed high-VBAR path can fetch. Do not change default `-cpu`.
7. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.
8. **ADRP is not a PA.** After VBAR is high, handler code that does `addr_of!(table)` / `user_ttbr0()` must mask to the 39-bit identity VA before programming `TTBR0` or comparing `FAR_EL1`. First attempts Failed: guard `FAR` compared to a high `__stack_guard`, then standing `stay_at_el0` programmed a high `L1_USER` as TTBR0. `paging::identity_pa` / `linker_sym` are the ratchet.

## Honesty

Say “EL1 fetched a real path from a TTBR1 high VA” or “VBAR lives at the high alias” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were torn down
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::init` installs the RAM alias and relocates VBAR. `src/ttbr1.rs` owns both the private-page probe and the exec probe.
- [ADR-016](ADR-016-ttbr1-private-page.md) remains the private-page first cut. This ADR is the exec mile.
- [ADR-018](ADR-018-identity-teardown-first-cut.md) splits TTBR1 RAM tables and unmaps one identity text page. [ADR-019](ADR-019-identity-text-range-tear.md) jumps the post-MMU continuation to high VA and unmaps a 16 KiB dedicated text range. Unmapping live `.text`/`.rodata`/`.data`/heap is still later.
