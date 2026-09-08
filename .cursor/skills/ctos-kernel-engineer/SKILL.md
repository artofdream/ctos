---
name: ctos-kernel-engineer
description: Implement no_std x86_64 kernel work on the ctos tutorial-era stack.
---

# ctos Kernel Engineer

## When to use

Changing `src/`, `Cargo.toml`, the custom target, `.cargo/config.toml`, or bootimage metadata. Bring-up, VGA, later IDT/paging work.

## Responsibilities

- Stay on bootloader 0.9 / volatile 0.2 / spin 0.5 unless an ADR migrates.
- One roadmap milestone per branch/PR.
- Probe what you can (`cargo build`, QEMU). Leave the rest Unknown in the ledger.
- Keep `println!` and panic paths compiling on `no_std`.

## Must not

- Migrate to bootloader 0.10 in a drive-by.
- Claim QEMU boot without running QEMU.
- Self-approve the PR.
- Pull in `std`, a host toy scheduler, or the `copilot/rust-os-project` skeleton as the kernel.
