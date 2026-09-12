# ADR-025 — Identity `.rodata` tear after pointer rewrite

- Status: Accepted (identity `.rodata` unmapped; `.data` / heap still identity-mapped)
- Date: 2026-09-12

## Context

Track A / A5 ([issue #36](https://github.com/artofdream/ctos/issues/36), parent [#31](https://github.com/artofdream/ctos/issues/31)) continues identity teardown **where safe**. [ADR-020](ADR-020-identity-fnptr-reloc.md) rewrote rustc vtables and unmapped live identity `.text` after the boot stub. `.rodata` / `.data` / heap stayed mapped: string literals and rewritten tables still lived there.

After the ADR-019 high-VA jump, new PC-relative accesses (ADRP) already reach the TTBR1 alias. `R_AARCH64_ABS64` words that *point at* `.rodata` stay identity until rewritten — the same class as the ADR-020 fn-pointer tables.

`.data` / `.bss` / linker stacks / heap are **not** safe to unmap here. SP is still the identity `__stack_top`. `GlobalAlloc` still returns identity frame VAs. Those stay Planned.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Rewrite identity pointers to `.rodata`, then unmap identity `.rodata` from kernel and user TTBR0. High twins stay. `.data` / heap stay. | Largest honest cut that does not relocate SP or change the allocator. |
| **H2 (rejected)** | Also unmap identity `.data` / heap. | SP and `Box`/`Vec` pointers are still identity VAs. A silent yank would fault. |
| **H3 (rejected)** | Claim PAN / “EL0 isolated” while tearing `.rodata`. | Isolation is not this mile. PAN is [ADR-026](ADR-026-pan-capability.md). |

## Decision

1. **Rewrite identity `.rodata` pointers.** After the live `.text` tear, walk `.rodata` and `.data`. Every 8-byte word that is an identity address in `[__rodata_start, __ident_tear_start)` is rewritten to its TTBR1 alias. RO pages are writable only for that store, then restored. Serial `ident: ro-reloc n=N`. `N == 0` is allowed (no ABS64 hits); the tear still has to prove high-only access.
2. **Unmap identity `.rodata`.** Unmap `[__rodata_start, __ident_tear_start)` from kernel and user TTBR0 (`TLBI VAAE1` each low VA). High twins stay. Serial `ident: rodata lo=… hi=… pages=N` with `N >= 1`.
3. **Keep `.data` / heap / linker stacks identity-mapped.** SP and the first-fit heap still use those VAs. Do not yank them here.
4. **Keep `_start` / QEMU `-kernel` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN.
5. **Fail-closed probe.** Existing `ident:*` markers stay. New `ident: ro-reloc` / `ident: rodata` / `ident: rodata-fault` / `ident: rodata-high`. EL1 `LDR` of a torn identity `.rodata` VA is a current-EL translation DABORT. EL1 `LDR` of the high twin still returns a known `.rodata` magic. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: rodata missed` / `ident: ro-reloc missed`. `#[test_case]` covers the unmap.
6. **Still Planned.** Identity `.data` / heap tear (needs SP relocate + high allocator VAs); PAN enable on `-cpu cortex-a57`; EL0 entry without `TLBI VMALLE1`; lower-EL IRQ while standing; umbrella EL0 isolation.
7. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “identity `.rodata` was unmapped while `println!` still ran from the high twin” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down (`.data` / heap stay)
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md); unimplemented on `-cpu cortex-a57`)

## Consequences

- `paging::rewrite_identity_rodata_ptrs` / `paging::tear_identity_rodata` own the cut. [ADR-020](ADR-020-identity-fnptr-reloc.md) remains the live `.text` tear.
- A later ADR may relocate SP and return high heap VAs, then unmap identity `.data` / heap. That work is not this cut.
