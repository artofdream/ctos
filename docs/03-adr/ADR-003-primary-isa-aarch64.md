# ADR-003 — Primary ISA is AArch64

- Status: Accepted
- Date: 2026-09-08

The sponsor brief named this decision “ADR-002.” On the harness tip, [ADR-002](ADR-002-pr-identity-split.md) is already the author ≠ merger identity split. This file is **ADR-003** so that accepted decision stays intact. Frozen [FR/NFR](../02-requirements/fr-nfr.md) IDs are unchanged; ISA-specific **text** is revised under this ADR.

## Context

The sponsor host (cts-ai) is Windows ARM64 with Docker `linux/arm64`. Native AArch64 QEMU is the right guest. Keeping x86_64 as primary would mean a translated or foreign ISA on that machine.

The previous primary path (phil-opp VGA text at `0xb8000`, `bootloader` 0.9, `bootimage`, `qemu-system-x86_64`) is superseded. It was a tutorial-era lift, not a host-ISA decision.

x86_64 may be noted later as a **secondary** target. It is **not** implemented in the change that lands this ADR.

## Decision

1. **Primary ISA is AArch64** (arm64). The checked-in custom target is `aarch64-ctos.json` (`aarch64-unknown-none` style: `os: none`, panic abort, static relocation, soft-float so early boot does not require enabling SIMD/FP in `CPACR_EL1`).
2. **Boot path** is QEMU `-kernel` of the kernel ELF on `-machine virt` (or `virt,gic-version=3`) with `-cpu cortex-a57` or `max`. Entry is `_start` at a linker-script address in virt RAM. No `bootloader` 0.9 crate and no `bootimage` disk as the primary path.
3. **Console** is the virt **PL011 UART** (MMIO `0x0900_0000`) via `print!` / `println!`. Not VGA `0xb8000`. `earlycon` remains an allowed later probe; this ADR does not claim it.
4. **Do not claim Raspberry Pi or other board support** until a board-specific probe exists. virt is the supported guest.
5. Delete the x86-only primary tree (`x86_64-ctos.json`, `src/vga_buffer.rs`) rather than leave a broken dual tree.

## Consequences

- New target JSON, linker script, and UART writer replace the VGA / bootloader 0.9 stack.
- `.cargo/config.toml` default target is `aarch64-ctos.json`. `json-target-spec` stays for current nightly.
- Roadmap M0/M1 probes are UART + `qemu-system-aarch64`, not a VGA dump at `0xb8e60`.
- FR-01, FR-02, FR-03, FR-05 (and other x86-specific wording) are revised in place. No new FR/NFR IDs.
- Later exception/interrupt work is VBAR / GIC, not IDT / PIC / TSS. M3 / [ADR-004](ADR-004-el1-vbar-brk.md) is the `VBAR_EL1` + `BRK` path.
- Honesty ledger rows for the x86 VGA path are historical. They do not verify the AArch64 path.
