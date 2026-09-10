# ADR-008 — Identity map and bump frame allocator

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-09](../02-requirements/fr-nfr.md) requires the kernel to read a firmware/QEMU memory map (DTB when probed) and establish paging / virtual memory. Roadmap M7 is that path. Heap `GlobalAlloc` is M8; the scheduler is M9.

QEMU `-machine virt` memory (see `hw/arm/virt.c`):

| Region | Physical |
| --- | --- |
| flash / boot ROM | `0x0000_0000` |
| GICv2 distributor / CPU interface | `0x0800_0000` / `0x0801_0000` |
| PL011 UART | `0x0900_0000` |
| RAM (`VIRT_MEM`) | `0x4000_0000`, default **128 MiB** |
| Kernel (`-kernel` TEXT_OFFSET `0x80000`) | `0x4008_0000` |
| DTB (typical `-kernel` placement) | RAM base `0x4000_0000`, below the kernel |

`_start` currently overwrites `x0` (the firmware DTB pointer) to set SP. A full FDT memory-node walk is extra surface for one milestone and is not required to prove MMU-on + allocate-frame on this machine.

Options:

1. **1 GiB L1 identity blocks** (39-bit VA, `T0SZ=25`, 4 KiB granule) plus a dedicated 4 KiB map window.
2. Fine-grained 4 KiB identity of every used page (more tables, same probe).
3. Higher-half kernel (breaks the current linker / VBAR / UART absolute addresses).
4. Parse the DTB before enabling the MMU (correct long-term; not the M7 fail-closed proof).

## Decision

1. **Enable the MMU at EL1** after `exception::init` (`ensure_el1`). Program `MAIR_EL1` (Attr0 Device-nGnRnE, Attr1 Normal WB), `TCR_EL1` (TTBR0 only, 40-bit IPS, inner-shareable WB), `TTBR0_EL1`, then set `SCTLR_EL1.M|C|I|SA`. Identity: VA == PA for everything the kernel already touches.
2. **L1 blocks, not a full 4 KiB identity.** Entry 0 (`0x0`–`1 GiB`) is Device-nGnRnE + XN (UART, GIC, flash). Entry 1 (`0x4000_0000`–`0x7FFF_FFFF`) was a Normal WB executable 1 GiB RAM block in M7. **[ADR-012](ADR-012-wx-nx-heap-stacks.md) supersedes that RAM *block*:** L1 slot 1 is now an L2 table, only 128 MiB is mapped, and `__kernel_end`…RAM-end is PXN. The allocator still must not hand out frames past `RAM_BASE + 128 MiB`.
3. **Frame pool from linker + virt convention.** `__kernel_end` (after image + stacks, 4 KiB-aligned) through `0x4000_0000 + 128 MiB`. Bump cursor plus a 16-entry free stack. Do not allocate `0x4000_0000`–`0x4008_0000` (DTB / TEXT_OFFSET hole). Smoke pins `-m 128M` so the pool matches the VM.
4. **Map/unmap probe uses a third GiB window** (`0x8000_0000`) via one L2 + L3 table, not by splitting the RAM block. Serial marker `paging: ok`. Do not load an unmapped VA (that would take an unhandled data abort).
5. **DTB parse is staged.** M7 does not walk FDT. FR-09’s “DTB when probed” stays Unknown until a later ADR. The probed map is the virt table above plus `__kernel_end`.
6. **No `GlobalAlloc`.** Frames are physical pages only.

## Consequences

- `scripts/qemu-smoke.sh` requires `paging: ok` and rejects `paging: probe missed`, then still requires M2–M6 strings.
- `#[test_case]` can assert `SCTLR_EL1.M`, distinct aligned frames, and a map/unmap write-through.
- Stack overflow was not a translation fault in M7 (RAM was one Normal block). Guard pages are [ADR-014](ADR-014-linker-stack-guard-pages.md), not this milestone.
- A later DTB walker or higher-half map needs a new ADR; it is not M8’s heap.
- This ADR does not claim Raspberry Pi, GICv3, EL0 user maps, or ASID isolation. Heap NX is [ADR-012](ADR-012-wx-nx-heap-stacks.md), not M7.
