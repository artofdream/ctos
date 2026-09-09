# Security (NFR-10 / ADR-011)

This file is a **threat-model stub**, not a complete model and not a “secure OS” claim. File presence is not W^X, not EL0 isolation, and not a passed audit.

## Assets (current QEMU `virt` guest)

- Kernel text and the `VBAR_EL1` table (EL1, identity-mapped).
- PL011 UART as the only console / smoke sensor.
- First-fit heap and cooperative task stacks (identity VA == PA).
- GICv2 + CNTP IRQ path.

## Adversaries (named so we do not invent them later)

- **Buggy kernel path** that writes executable RAM (today the 1 GiB RAM L1 block is W+X).
- **Later EL0** (not built): a user task executing kernel data or escalating via a bad map.
- **Host/CI secrets**: credentials committed to git. Out of scope for the guest; in scope for the repo.

Out of scope for now: secure boot, measured boot, multi-tenant isolation, networking, Raspberry Pi, a second ISA.

## Current posture (honest)

| Control | State | Notes |
| --- | --- | --- |
| No secrets in repo | Policy (NFR-10) | Must. Not a guest hardening claim. |
| Minimize `unsafe` | Policy (NFR-01) | Hardware/FFI/allocator boundaries only. Count is not a proof. |
| Device MMIO XN | Source | L1 block 0 is Device-nGnRnE + UXN/PXN ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)). |
| Heap / stacks NX (W^X) | **Planned** | RAM L1 block 1 is executable. Heap and stacks live there. Needs an L2/L3 split. |
| Execute-from-writable heap | **Planned** to forbid | Same identity block. Do not claim NX because the M7 map window can set PXN — that window is not the heap. |
| IRQ least privilege | Convention | IRQ handler does not allocate or `yield_now`. No execute-from-IRQ scheduler. Needs a ratchet if this fails twice. |
| EL0 isolation | **Planned** | Vision Out list until a later ADR. |

## Claim gate

A PR may say “threat-model stub exists” after a file read. It may **not** say “secure,” “hardened,” “W^X,” or “NX heap” unless the honesty ledger has a matching probe. Unprobed stays **Unknown**.
