# ADR-037 — Identity `.data` / `.bss` / linker-stack tear after SP relocate

- Status: Accepted (identity `.data`/`.bss`/linker stacks unmapped; heap still identity-mapped)
- Date: 2026-09-12

## Context

Track A / A5 continues identity teardown **where safe**. [ADR-025](ADR-025-identity-rodata-tear.md) unmapped identity `.rodata` after a pointer rewrite. `.data` / `.bss` / linker stacks / heap stayed mapped: SP was still the identity `__stack_top`, page-table software walked tables via identity PAs that live in `.bss`, and `GlobalAlloc` still returned identity frame VAs.

After the ADR-019 high-VA jump, PC-relative accesses (ADRP) already reach the TTBR1 alias of statics. Absolute words that *point into* `.data`/stacks stay identity until rewritten. SP and SP_EL1 must move to the high twin of the same physical pages before those identity pages can be unmapped. Heap tear needs a high allocator and is **not** this cut.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Rewrite identity pointers into `.data`/stacks, relocate SP/SP_EL1 to high twins, switch page-table CPU accessors to the high twin, then unmap identity `[__data_start, __kernel_end)` from kernel+user TTBR0. High twins stay. Heap stays. | Largest honest cut that does not change `GlobalAlloc`. |
| **H2 (rejected)** | Also unmap identity heap. | Allocator still returns identity VAs. A silent yank would fault. |
| **H3 (rejected)** | Claim PAN / “EL0 isolated” / “kernel moved” while tearing `.data`. | Isolation is not this mile. PAN is [ADR-026](ADR-026-pan-capability.md). `_start` stays at `0x4008_0000`. |
| **H4 (rejected)** | Unmap `.data` without relocating SP or fixing PA-as-pointer table walks. | Would fault on the next store or `l3_write`. |

## Decision

1. **Rewrite identity `.data` pointers first.** After the live `.text` tear and **before** the `.rodata` tear (so identity `.rodata` is still walkable), walk `.rodata` and `[__data_start, __kernel_end)`. Every 8-byte-aligned word that is an identity address in that data/stack range is rewritten to its TTBR1 alias. Valid page-table descriptors are excluded (low bits set). Serial `ident: data-reloc n=N`. `N == 0` is allowed.
2. **Keep the ADR-025 `.rodata` cut.** `ident: ro-reloc` / `ident: rodata` unchanged.
3. **Relocate SP.** Move the current SP (SP_EL0) and SP_EL1 to `to_high_va` of the same physical stack pages. High guard holes already exist in `L2_HIGH_RAM` (cloned after identity guards). Serial path continues only if SP is high.
4. **Page-table CPU VA.** After the high split, `desc_at` / `l3_write` / flag publish-load use the TTBR1 twin of table/flag PAs so identity `.bss` can be torn while the hardware walk still uses PAs.
5. **Unmap identity `.data`/`.bss`/linker stacks.** Unmap `[__data_start, page_align_up(__kernel_end))` from kernel and user TTBR0 (`TLBI VAAE1` each low VA). High twins stay. Serial `ident: data lo=… hi=… pages=N` with `N >= 1`.
6. **Keep heap identity-mapped.** Frames at/after `__kernel_end` stay. Print `ident: heap-stay`. Do not claim a high allocator.
7. **Keep `_start` / QEMU `-kernel` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN.
8. **Fail-closed probe.** New `ident: data-reloc` / `ident: data` / `ident: data-fault` / `ident: data-high`. EL1 `LDR` of a torn identity `.data` VA is a current-EL translation DABORT. EL1 `LDR` of the high twin still returns a known `.data` magic. `scripts/qemu-smoke.sh` greps those strings and rejects missed markers. Replace `ident: data-stay`. Split `#[test_case]` into heap-still-mapped vs data-torn.
9. **Still Planned (at accept time).** Identity heap tear (high allocator VAs). Superseded by [ADR-038](ADR-038-identity-heap-tear.md). PAN enable on `-cpu cortex-a57`; EL0 entry without `TLBI VMALLE1`; lower-EL IRQ while standing; umbrella EL0 isolation.
10. **NFR-10 text** is revised in place (ID unchanged). Do not mint NFR-15+.

## Honesty

Say “identity `.data`/`.bss`/linker stacks were unmapped while the high twin and heap stayed” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down (heap stays; `_start` stays)
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md); unimplemented on `-cpu cortex-a57`)

## Consequences

- `paging::rewrite_identity_data_ptrs` / `paging::relocate_stacks_high` / `paging::tear_identity_data` own the cut. [ADR-025](ADR-025-identity-rodata-tear.md) remains the `.rodata` tear. [ADR-032](ADR-032-track-a-leftovers.md) stay markers for `.data` are superseded by this tear; heap stay markers remain.
- [ADR-038](ADR-038-identity-heap-tear.md) returns high heap VAs and unmaps identity heap. That work is not this cut.
