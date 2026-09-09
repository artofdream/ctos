# ADR-005 — Dedicated exception and fatal stacks

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-07](../02-requirements/fr-nfr.md) requires a fatal exception to use a dedicated stack so overflow does not silently lock the VM. Roadmap M4 is that path. [ADR-004](ADR-004-el1-vbar-brk.md) already notes that a nested fault while `println!` holds the UART mutex can deadlock.

M3 ran the kernel and first-level current-EL exceptions on the same `SP_EL1` (SPSel = 1). A nested sync exception would store another 272-byte frame on that stack. Without paging a downward overflow is not a hardware fault on QEMU `virt` — it smashes `.bss` / code with no serial evidence. M7 identity-maps RAM as one Normal block, so this is still true (no guard pages).

A data abort to an unused physical hole is QEMU-map-dependent. A nested `BRK` from the first-level handler is a real current-EL exception we already know how to take (FR-06).

## Decision

1. **Thread stack on `SP_EL0`.** After `VBAR_EL1` is installed, copy the `_start` stack into `SP_EL0` (`MSR SP_EL0` is legal at EL1), write `__exc_stack_top` into `SP` while `SPSel` is still 1 (that *is* `SP_EL1` — `MSR SP_EL1` at EL1 is UNDEF), then `msr spsel, #0`. Normal kernel code uses the 64 KiB thread stack.
2. **Exception stack on `SP_EL1`.** First-level current-EL exceptions (vector bank “Current EL, SP_EL0”) use the 16 KiB `__exc_stack_*` region automatically. The live sync slot moves from offset `0x200` (M3 / SP_ELx) to `0x000`.
3. **Fatal stack before any nested store.** Current-EL / SP_ELx slots (`0x200`–`0x380`) load `SP` from `__fatal_stack_top` (8 KiB) in asm, then call `handle_fatal_exception`. Do not push a frame on the exception stack that may already be exhausted.
4. **Raw UART on fatal / unhandled paths.** Write the PL011 without `spin::Mutex` so a nest during `println!` still produces serial.
5. **Probe is nested `BRK` after a near-empty thread SP.** The hello kernel fires a healthy-stack `BRK` (M3 string), then sets a flag, moves `SP_EL0` to `__stack_bottom + 64` (less than the 272-byte frame), and `BRK`s again. The first-level handler prints `exception: sync BRK` from `SP_EL1`, then `BRK`s while SPSel = 1. That nest must print `exception: fatal nested` and park. This is not an MMU stack-overflow fault and not a GIC path (M5 / FR-08).

## Consequences

- `#[test_case]` can still `brk #0` and return. Tests do not set the nest flag.
- `scripts/qemu-smoke.sh` requires the fatal marker in addition to hello + BRK. A `fatal probe missed` line is fail-closed.
- Lower-EL and first-level FIQ/SError stubs still park. First-level IRQ is M5 / [ADR-006](ADR-006-gicv2-generic-timer.md). Nested IRQ/FIQ/SError still use the fatal stack. Taking a nested IRQ remains unprobed.
- This ADR does not claim Raspberry Pi, paging guard pages, or x86.
