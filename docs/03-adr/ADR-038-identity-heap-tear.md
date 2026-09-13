# ADR-038 — Identity heap tear after high GlobalAlloc VAs

- Status: Accepted (identity heap unmapped; `GlobalAlloc` returns TTBR1 VAs) when the serial / tests pass
- Date: 2026-09-12

## Context

Track A / A5 continues identity teardown **where safe**. [ADR-037](ADR-037-identity-data-tear.md) unmapped identity `.data`/`.bss`/linker stacks after SP relocate. Heap stayed identity-mapped: `frame::alloc_contiguous` handed out PAs and `GlobalAlloc` returned those identity VAs (`ident: heap-stay`).

Heap `init` already runs **after** the high-VA jump and the `.data` tear. High twins of the frame pool exist (`L2_HIGH_RAM` clone). The next honest cut is to install the first-fit pool at the TTBR1 alias, rewrite leftover identity heap pointers, then unmap the identity heap range. PAN enable and umbrella EL0 isolation are **not** this cut.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Install the heap free list at `to_high_va(pa)`, rewrite leftover identity heap words, then unmap identity `[heap_pa, heap_pa+HEAP_SIZE)` from kernel+user TTBR0. High twins stay. Remaining frames after the heap stay identity-mapped. | Smallest honest tear that does not yank live identity VAs. |
| **H2 (rejected)** | Unmap identity heap while `GlobalAlloc` still returns identity VAs. | Silent yank would fault. |
| **H3 (rejected)** | Claim PAN / “EL0 isolated” / “kernel moved” while tearing the heap. | Isolation is not this mile. PAN is [ADR-026](ADR-026-pan-capability.md). `_start` stays at `0x4008_0000`. |
| **H4 (rejected)** | Remap only new allocations high and leave the existing identity pool mapped. | Would print a stay marker forever. |

## Decision

1. **High allocator first.** After `frame::init`, `heap::init` takes the same 16-frame run and installs the free list at the TTBR1 alias. `GlobalAlloc` returns high VAs. Serial later prints `ident: heap-reloc n=N`. `N == 0` is allowed (init already high).
2. **Rewrite leftover identity heap pointers.** Walk the heap pool via the high twin (do not walk live stacks or page-table `.bss`). Every 8-byte-aligned word that is an identity address in the heap PA range is rewritten to its TTBR1 alias. `heap::init` already installed high VAs, so `n == 0` is the expected path. A leaked `Box` plants `ident` magic for the high-twin load.
3. **Unmap identity heap.** Unmap `[heap_pa, heap_pa+HEAP_SIZE)` from kernel TTBR0 (`TLBI VAAE1` each low VA). User TTBR0 already omits heap; clearing a present slot is fine. High twins stay. Frames after the heap stay identity-mapped. Serial `ident: heap lo=… hi=… pages=N` with `N >= 1`.
4. **Keep `_start` / QEMU `-kernel` at `0x4008_0000`.** Do not change default `-cpu`. Do not claim PAN.
5. **Fail-closed probe.** New `ident: heap-reloc` / `ident: heap` / `ident: heap-fault` / `ident: heap-high`. EL1 `LDR` of a torn identity heap VA is a current-EL translation DABORT. EL1 `LDR` of the high twin still returns the planted free-list header size. `scripts/qemu-smoke.sh` greps those strings and rejects `ident: heap-stay`. Replace the ADR-032 / ADR-037 stay marker.
6. **Still Planned (at accept time).** PAN enable on `-cpu cortex-a57`; EL0 entry without `TLBI VMALLE1`; lower-EL IRQ while standing; umbrella EL0 isolation. Remaining identity RAM after the heap (frame bump pool) is not this cut. EL0 entry without `VMALLE1` is superseded by [ADR-039](ADR-039-el0-entry-without-vmalle1.md).
7. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.18** at accept (v1.19 after ADR-039). Do not mint NFR-15+.

## Honesty

Say “identity heap was unmapped while the high twin stayed and `GlobalAlloc` returned TTBR1 VAs” only when the serial / tests pass. Do **not** say:

- the kernel has moved to the high half
- identity mappings were fully torn down (`_start` stays; frame pool after the heap stays identity-mapped)
- “secure OS” / “hardened” / “EL0 isolated”
- PAN (see [ADR-026](ADR-026-pan-capability.md); unimplemented on `-cpu cortex-a57`)

## Consequences

- `heap::init` / `paging::rewrite_identity_heap_ptrs` / `paging::tear_identity_heap` own the cut. [ADR-009](ADR-009-first-fit-heap.md) stays the first-fit algorithm; VAs are now high. [ADR-037](ADR-037-identity-data-tear.md) stay marker for the heap is superseded.
- A later ADR may tear remaining identity frames or enable PAN. That work is not this cut.
