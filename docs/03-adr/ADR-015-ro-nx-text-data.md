# ADR-015 — RO+NX text / data split

- Status: Accepted
- Date: 2026-09-11

## Context

[ADR-012](ADR-012-wx-nx-heap-stacks.md) made the heap and heap-backed cooperative stacks PXN. [ADR-014](ADR-014-linker-stack-guard-pages.md) punched unmapped holes under the linker stacks. Both left `.text` / `.rodata` / `.data` / `.bss` / live linker-stack **pages** in one executable, writable image mapping. `SCTLR_EL1.WXN` was unusable while text was writable.

[NFR-10](../02-requirements/fr-nfr.md) still forbids saying “the kernel is W^X” without a scoped probe. This ADR is the next cut: split RO+X from RW+NX.

## Decision

1. **Page-align `__data_start`** in `linker.ld` so `.text`/`.rodata` never share a 4 KiB page with `.data`.
2. **Identity flags (still 39-bit / 4 KiB / TTBR0):**
   - `[KERNEL_TEXT, __data_start)` — RO+X (AP[2]=1, PXN clear, UXN set)
   - `[__data_start, RAM end)` — RW+NX (including `.data`, `.bss`, live linker stacks, frame pool / heap)
   - Device L1 stays XN. Guard holes stay invalid ([ADR-014](ADR-014-linker-stack-guard-pages.md)).
3. **`SCTLR_EL1.WXN` on** once text is RO. Writable pages are treated as XN even if a descriptor forgets PXN.
4. **Fail-closed probe:** execute-from-`.data` (`blr` to a `RET` bait) is a current-EL permission IABORT (`ro: nx data`). Store to RO text is a current-EL permission DABORT (`ro: write fault`). Serial `ro: ok`. `scripts/qemu-smoke.sh` greps those strings and rejects `ro: probe missed`. `#[test_case]` covers flags + both faults.
5. **NFR-10 text** is revised in place (ID unchanged): mention the RO+NX image cut. Do not mint NFR-15+.

## Honesty — is this “the kernel is W^X”?

On **this QEMU `virt` guest**, the identity image plus heap is W^X: text/rodata are RO+X, data/bss/live linker stacks/heap are RW+NX, WXN is on. That is **not**:

- a “secure OS” / “hardened” claim
- a promise that every future mapping (DMA, new windows) stays W^X
- Raspberry Pi or a second ISA
- a reason to drop the heap / guard probes

Say “identity image is W^X on this virt guest (ADR-015 probe)” only when the serial / tests pass. Do not say “the kernel is W^X” as a product sentence.

## Consequences

- `src/paging.rs` sets AP[2] on text and PXN on data/stacks. `src/ro.rs` owns the two faults.
- Live linker stacks are NX here. If a later change must execute from a linker stack, this ADR is the one to revisit.
- EL0 user TTBR0 (ADR-013 mile on this PR) still maps kernel text so the lower-EL handler can restore kernel TTBR0; it omits `.data`. That is isolation-adjacent, not this ADR.
- This ADR does not claim PAN, ASID isolation, or a higher-half kernel.
