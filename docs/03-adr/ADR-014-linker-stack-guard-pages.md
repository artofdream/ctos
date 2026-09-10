# ADR-014 — Unmapped guard pages under linker stacks

- Status: Accepted
- Date: 2026-09-10

## Context

[ADR-012](ADR-012-wx-nx-heap-stacks.md) made the heap and heap-backed cooperative stacks PXN. It explicitly left `SP_EL0` / `SP_EL1` / fatal stacks in the **executable** kernel image and named stack guard pages as a later ADR. [ADR-005](ADR-005-fatal-exception-stack.md) still mitigates overflow with a dedicated fatal stack and a nested-`BRK` probe — that is not a translation fault.

A downward overflow of a linker stack today smashes the previous image bytes (`.bss` or the stack above). QEMU `virt` will not notice.

Options:

1. **Unmapped 4 KiB guard page(s)** below each linker stack. Overflow becomes a current-EL data abort (translation). Observable serial + `#[test_case]`.
2. **PXN on stack pages** after splitting them from text. Stops execute-from-stack; overflow still smashes mapped data unless we also unmap something.
3. **`SCTLR_EL1.WXN`** — still unusable while kernel text pages are writable.

## Decision

1. **Option 1.** Insert a 4 KiB linker hole below the thread, exception, and fatal stacks (`__stack_guard`, `__exc_stack_guard`, `__fatal_stack_guard`). After `fill_ram_wx`, split any 2 MiB L2 block that contains a guard into L3 and **clear that L3 slot** (invalid). Identity VA == PA is unchanged.
2. **Fail-closed probe:** arm a current-EL translation data-abort catch, store to the thread-stack guard, resume at LR. Serial `guard: fault` + `guard: ok`. `scripts/qemu-smoke.sh` greps `guard: ok` and rejects `guard: probe missed`.
3. **Do not claim linker-stack NX or “the kernel is W^X.”** Guard pages are holes. The live stack pages stay executable with text / `.data` / `.bss`. A later RO+NX split or `SCTLR.WXN` needs another ADR.
4. **NFR-10 text** is revised in place (ID unchanged): mention the guard-page overflow cut. Do not mint NFR-15+.
5. The ADR-005 nested-`BRK` fatal probe stays. It does not write below `__stack_bottom`.

## Consequences

- `src/paging.rs` may add one or two extra L3 tables to split an executable L2 block that is not the `__kernel_end` straddle.
- Overflow of a *heap-backed* cooperative stack is still not this probe (those stacks live in PXN frame-pool RAM with no guard holes).
- This ADR does not claim a secure OS, Raspberry Pi, or ASAN-quality stack checking.
