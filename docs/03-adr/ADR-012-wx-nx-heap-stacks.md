# ADR-012 — W^X: NX heap and cooperative stacks

- Status: Accepted
- Date: 2026-09-10

## Context

[NFR-10](../02-requirements/fr-nfr.md) and [ADR-011](ADR-011-three-pillars.md) require a probe before any W^X claim. [ADR-008](ADR-008-identity-map-frame-allocator.md) mapped virt RAM as **one executable 1 GiB L1 Normal block**. The heap ([ADR-009](ADR-009-first-fit-heap.md)) and cooperative stacks ([ADR-010](ADR-010-cooperative-rr-el1.md)) live in that block, so they were W+X. Setting PXN only on the M7 map window (`0x8000_0000`) would have been a fake “NX heap” claim.

Device MMIO (L1 block 0) was already XN. The missing cut is: **heap and heap-backed stacks NX, kernel text still executable**.

`SCTLR_EL1.WXN` is not usable while kernel text pages are writable — it would make text NX too.

## Decision

1. **Replace the RAM L1 *block* with an L2 table** (still 39-bit / 4 KiB / TTBR0). Map only the 128 MiB guest (`0x4000_0000`…`+128 MiB`). The rest of that GiB stays invalid.
2. **2 MiB L2 blocks** where a block is entirely kernel-image or entirely frame-pool. **One L3 table** for the single 2 MiB that straddles `__kernel_end`.
3. **Executable:** pages in `[0x4008_0000, align_4k(__kernel_end))` (text, rodata, data, BSS, linker stacks). **PXN+UXN:** `[__kernel_end, RAM end)` (frame pool, heap, cooperative stacks) and the DTB hole below `0x4008_0000` when that hole is in the straddling L3. Device L1 stays XN. The M7 map window L3 pages are PXN (data-only).
4. **Do not claim the linker stacks NX.** SP_EL0 / SP_EL1 / fatal stacks sit in the executable image. Overflow there is still not a translation fault (ADR-005).
5. **Fail-closed probe:** walk PXN on kernel text (clear) and heap (set), then write `RET` on the heap, `blr` to it, and catch a current-EL permission **instruction abort**. Serial `wx: nx heap` + `wx: ok`. `scripts/qemu-smoke.sh` greps `wx: ok` and rejects `wx: probe missed`.
6. **NFR-10 text** is revised in place (ID unchanged): heap + coop-stack NX is the scoped W^X claim, not “the kernel is W^X.”

## Consequences

- `src/paging.rs` grows `L2_RAM` / `L3_RAM`. Identity VA == PA is unchanged.
- A later RO+NX split of `.text` vs `.data`, stack guard pages, or `SCTLR.WXN` needs a new ADR.
- EL0 isolation is [ADR-013](ADR-013-el0-isolation-direction.md) (Planned). UXN is already set on RAM so a future EL0 cannot fetch kernel or heap by accident; that is not an EL0 probe.
- This ADR does not claim a secure OS, Raspberry Pi, or side-channel resistance.
