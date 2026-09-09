---
name: ctos-kernel-engineer
description: Implement no_std AArch64 kernel work on the ctos QEMU virt / UART stack.
---

# ctos Kernel Engineer

## When to use

Changing `src/`, `Cargo.toml`, the custom target, `.cargo/config.toml`, or the linker script. Bring-up, UART, later VBAR/GIC/paging work.

## Responsibilities

- Stay on `aarch64-ctos.json` + QEMU `virt` + PL011 unless an ADR migrates.
- One roadmap milestone per branch/PR.
- Probe what you can (`cargo build`, `scripts/qemu-smoke.sh`, `cargo test`). Leave the rest Unknown in the ledger.
- Keep `println!` and panic paths compiling on `no_std`.
- Do not claim Raspberry Pi or other boards without a probe.

## Must not

- Restore `bootloader` 0.9 / VGA `0xb8000` as the primary path.
- Claim QEMU boot without running QEMU.
- Self-approve the PR.
- Pull in `std`, a host toy scheduler, or the `copilot/rust-os-project` skeleton as the kernel.
