# Session memory — 2026-09-08

Sponsor switched primary ISA to arm64. ADR-002 was already identity split, so the ISA decision is ADR-003. Deleted `x86_64-ctos.json` and `src/vga_buffer.rs`. Soft-float custom target to avoid early CPACR_EL1. rustc 1.100 required both `abi` and `rustc-abi` = softfloat. virt PL011 hello probed on QEMU 8.2.2; no raspi claim.
