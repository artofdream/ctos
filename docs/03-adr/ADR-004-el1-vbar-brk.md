# ADR-004 — EL1 VBAR and resumable BRK

- Status: Accepted
- Date: 2026-09-09

## Context

[FR-06](../02-requirements/fr-nfr.md) requires synchronous exceptions via `VBAR_EL1` (at least a breakpoint / fault path). Roadmap M3 is that path. M4 (FR-07) is a dedicated fatal stack. M5 (FR-08) is GIC IRQs.

QEMU `virt` `-kernel` usually starts the guest at EL1. `-machine virt,virtualization=on` starts at EL2. Exceptions taken to the current EL use that EL’s `VBAR_*`. Staying at EL2 and programming only `VBAR_EL2` would not satisfy FR-06.

## Decision

1. **Live at EL1.** `exception::init` calls `ensure_el1`: if `CurrentEL` is EL2, set `HCR_EL2.RW`, copy `SP` to `SP_EL1`, and `ERET` to EL1h (DAIF masked). If the EL is not 1 after that, print and park. Do not treat `VBAR_EL2` as the primary table.
2. **One 2 KiB-aligned table** at `exception_vectors`, written to `VBAR_EL1`. All sixteen AArch64 slots exist. Current EL / SP_ELx / synchronous saves a frame and may `ERET`. Every other slot parks (UART line + `wfe`, or semihosting fail under `cargo test` / `force-fail`).
3. **Context format** (current-EL sync only): `x0`–`x29`, `x30`, `ELR_EL1`, `SPSR_EL1`, `ESR_EL1`. No SIMD/FP save (soft-float, [ADR-003](ADR-003-primary-isa-aarch64.md)).
4. **`BRK` is resumable.** ESR exception class `0x3C` (AArch64 `BRK`) increments a counter, prints `exception: sync BRK`, adds 4 to `ELR_EL1`, and returns. Other synchronous exceptions are fatal for this milestone (print + park). That is not FR-07’s dedicated overflow stack.

## Consequences

- `#[test_case]` can execute `brk #0` and continue. The hello kernel fires one `BRK` so `scripts/qemu-smoke.sh` can require the handler string on serial.
- Lower-EL and IRQ/FIQ/SError stubs are parks. They are not a syscall ABI or a timer (M5).
- A nested fault while `println!` holds the UART mutex can deadlock. [ADR-005](ADR-005-fatal-exception-stack.md) (M4 / FR-07) splits `SP_EL0` / `SP_EL1`, adds a fatal stack, and uses a raw UART write on that path.
- This ADR does not claim Raspberry Pi, EL0, or a taken lower-EL exception.
