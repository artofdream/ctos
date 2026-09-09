# Session memory — 2026-09-09 (M4)

Without MMU, overflowing SP is silent corruption, not a fault. Honest virt probe: drop `SP_EL0` to `__stack_bottom+64` (< 272-byte frame) then `BRK`; first-level handler runs on `SP_EL1` and nests a second `BRK` (SPSel=1 → SP_ELx bank → `__fatal_stack_top` before any store). Raw UART: mutex can already be held. Do not implement GIC here. Do not claim Raspberry Pi.
