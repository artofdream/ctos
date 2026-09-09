# Technical architecture

ctos is a `#![no_std]` `#![no_main]` binary. There is no Rust standard library and no OS underneath. The crate builds for a **custom target** (`aarch64-ctos.json`): `os: none`, panic abort, red zone off, static relocation, soft-float (no early SIMD/FP). See [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md).

## Boot path

1. `qemu-system-aarch64 -machine virt -cpu cortex-a57` (or `max`; `gic-version=3` is allowed).
2. `-kernel` loads the kernel ELF into virt RAM (linker base `0x40080000`).
3. Entry is `_start` (assembly in `src/main.rs`): set SP, zero BSS, call `kernel_main`.
4. Early output is the virt PL011 UART at `0x0900_0000` (`src/uart.rs`).
5. `kernel_main` installs `VBAR_EL1` (`src/exception.rs`, [ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)) after UART init.

There is no `bootloader` 0.9 crate and no VGA buffer. The x86_64 phil-opp path was deleted when this ADR landed.

`.cargo/config.toml` still needs `json-target-spec = true` on rustc 1.100 nightly. The AArch64 JSON is taken from `aarch64-unknown-none-softfloat` plus `os: none` / numeric widths. That nightly rejected the file until both `"abi": "softfloat"` and `"rustc-abi": "softfloat"` were set. Behavior is bare-metal AArch64, abort, `rust-lld`.

This architecture does **not** claim Raspberry Pi or other SoC support.

## Current stage (UART hello + M2 tests + M3 VBAR + M4 fatal stack)

- `Pl011` writer with TX-full wait and `\n` → `\r\n`
- `print!` / `println!` via `spin::Mutex`; fatal / unhandled paths write the PL011 without the mutex
- `kernel_main` prints `Hello World!`, fires one healthy-stack `BRK #0` (serial `exception: sync BRK`), then the FR-07 nest probe (serial `exception: fatal nested`)
- `VBAR_EL1` vector table; kernel runs on `SP_EL0`; first-level current-EL sync (SP_EL0 bank) handles AArch64 `BRK` and returns; nested current-EL (SP_ELx bank) switches to the fatal stack ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md), [ADR-005](../03-adr/ADR-005-fatal-exception-stack.md))
- `cargo test` uses `#![feature(custom_test_frameworks)]` and `#[test_case]` (including VBAR, BRK, SPSel, stack ranges)
- QEMU exit is ARM **semihosting** `SYS_EXIT` / `hlt #0xf000` (`src/qemu.rs`), not `isa-debug-exit`. Needs `-semihosting` on the QEMU line (`scripts/qemu-aarch64.sh`).
- Host smoke: `scripts/qemu-smoke.sh` (hello + BRK + fatal nested strings + tests + `force-fail` must be non-zero)
- Docker: `Dockerfile` / `scripts/docker-smoke.sh` (linux/arm64-friendly; do not pin amd64)
- GHA: `.github/workflows/smoke.yml` (`ubuntu-24.04-arm` and `ubuntu-24.04`)

Source + local smoke were probed on 2026-09-08 (see the honesty ledger). GHA `smoke.yml` was green on that revision (`ubuntu-24.04` and `ubuntu-24.04-arm`). cts-ai Docker Desktop linux/arm64 `docker run --rm ctos-smoke` was Verified on 2026-09-09 after the LF / `cc` / ROM ratchets.

## Planned stages

| Stage | Domain work |
| --- | --- |
| Custom test framework | Landed (M2): `#[test_case]`, semihosting exit, UART |
| CPU exceptions | M3: `VBAR_EL1`, resumable `BRK`. M4: dedicated exception + fatal stacks (FR-07) — probes in the honesty ledger |
| Hardware interrupts | GIC, timer, later input |
| Paging | page tables, frame allocator |
| Heap | `alloc`, a simple allocator |
| Scheduler | cooperative or round-robin tasks |

Each stage is one loop unit on the [roadmap](../04-roadmap/roadmap.md).

x86_64 remains a possible **future secondary** ISA. It is not a current tree.

## Harness mapping (short)

Hardware and QEMU are the **domain**. Docs, ADRs, and this architecture note are **shared understanding**. Guides, sensors, the one-PR loop, second-brain vaults, merge permissions, and the honesty ledger are the **outer harness**. Details: [formula.md](../framework/formula.md).
