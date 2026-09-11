# ADR-018 — Identity teardown first cut (split tables + one torn text page)

- Status: Accepted (partial tear; full identity teardown still Planned)
- Date: 2026-09-11

## Context

[ADR-017](ADR-017-ttbr1-high-el1-exec.md) proved EL1 can fetch a real path from the TTBR1 RAM alias and moved `VBAR_EL1` there. The alias reused identity `L2_RAM`, so unmapping a low page also dropped the high twin. `_start` / QEMU `-kernel` still load at `0x4008_0000`. rustc emits link-time identity addresses (`relocation-model: static`), so a complete high-VA jump of `kernel_main` plus unmap of all identity `.text`/`.data`/heap is still too large for one honest PR.

This ADR is the **largest Verified cut** that still boots on virt: split the RAM tables, then unmap one defined identity text page that EL1 no longer needs for fetch.

## Decision

1. **Split RAM tables.** After identity W^X + stack guards, clone `L2_RAM` (and every L3 it points at) into `L2_HIGH_RAM` / `L3_HIGH_RAM`. `L1_HIGH[1]` points at the clone. L2 block descriptors are copied; L3 tables are duplicated. Unmapping an identity page no longer unmaps the high twin.
2. **Dedicated tear page.** The linker keeps a 4 KiB `ident_tear` section (`__ident_tear_start` … `__ident_tear_end`) in the RO+X image, after `.rodata` and before `__data_start`. It is **not** the `0x4008_0000` `_start` page.
3. **Unmap after MMU + high VBAR.** `paging::init` turns the MMU on, programs the high VBAR, then clears the identity and user-TTBR0 L3 slots for that page and `TLBI VAAE1`s the low VA. The high twin stays PXN-clear.
4. **Fail-closed probe.** Serial `ident: split` (clone + torn low / live high walks). EL1 `BLR` of the low VA is a current-EL translation IABORT (`ident: fault`). EL1 `BLR` of the high twin still runs `ident_tear_el1_path` (`ident: high`). EL0 `LDR` of the low VA is a lower-EL translation DABORT (`ident: no el0`). Serial `ident: ok`. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: probe missed` / `ident: leaked`.
5. **Identity boot stub stays.** `_start`, QEMU `-kernel` load, remaining identity `.text`/`.data`/heap, PL011 `0x0900_0000`, and later probes keep using TTBR0 identity VAs. **Do not say the kernel moved.**
6. **Still Planned.** Full identity teardown (unmap all low `.text`/`.data`/heap after a complete high-VA jump); PAN on `-cpu cortex-a57`; umbrella EL0 isolation. Do not change default `-cpu`.
7. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “TTBR1 RAM tables are independent of identity” or “one identity text page was unmapped while EL1 still fetched the high twin” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (still unclaimed on `-cpu cortex-a57`)

## Consequences

- `paging::init` clones RAM tables and unmaps `__ident_tear_*`. `src/teardown.rs` owns the serial probe.
- [ADR-017](ADR-017-ttbr1-high-el1-exec.md) remains the exec mile. This ADR is the first identity-teardown cut.
- A later ADR may jump `kernel_main` to high VA and unmap the rest of identity. That work is not this cut.
