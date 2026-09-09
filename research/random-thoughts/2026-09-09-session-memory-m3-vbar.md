# Session memory — 2026-09-09 (M3)

VBAR must be 2 KiB-aligned (linker `ALIGN(2048)` + `.align 11`). Current EL uses SP_ELx (`SPSel=1` after `_start`). ESR EC `0x3C` is AArch64 `BRK`; skip 4 bytes. QEMU `virt` without `virtualization=on` is usually already EL1; `ensure_el1` still drops EL2→EL1h so FR-06 stays `VBAR_EL1`. No FP/SIMD save (softfloat). Nested fault while UART mutex is held can deadlock — M4 problem. Do not claim Raspberry Pi.
