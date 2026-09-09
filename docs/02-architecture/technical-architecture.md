# Technical architecture

ctos is a `#![no_std]` `#![no_main]` binary. There is no Rust standard library and no OS underneath. The crate builds for a **custom target** (`aarch64-ctos.json`): `os: none`, panic abort, red zone off, static relocation, soft-float (no early SIMD/FP). See [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md).

## Boot path

1. `qemu-system-aarch64 -machine virt -cpu cortex-a57` (or `max`; `gic-version=3` is allowed).
2. `-kernel` loads the kernel ELF into virt RAM (linker base `0x40080000`).
3. Entry is `_start` (assembly in `src/main.rs`): set SP, zero BSS, call `kernel_main`.
4. Early output is the virt PL011 UART at `0x0900_0000` (`src/uart.rs`).
5. `kernel_main` installs `VBAR_EL1` (`src/exception.rs`, [ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)) after UART init, then the frame pool + EL1 identity map ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)), then the first-fit heap ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)), then the cooperative scheduler ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)), then GICv2 + CNTP ([ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md)). After `Hello World!` it proves map/unmap, `Box`/`Vec`, two tasks, one timer tick, and polls PL011 RX ([ADR-007](../03-adr/ADR-007-pl011-uart-rx.md)).

There is no `bootloader` 0.9 crate and no VGA buffer. The x86_64 phil-opp path was deleted when this ADR landed.

`.cargo/config.toml` still needs `json-target-spec = true` on rustc 1.100 nightly. The AArch64 JSON is taken from `aarch64-unknown-none-softfloat` plus `os: none` / numeric widths. That nightly rejected the file until both `"abi": "softfloat"` and `"rustc-abi": "softfloat"` were set. Behavior is bare-metal AArch64, abort, `rust-lld`.

This architecture does **not** claim Raspberry Pi or other SoC support.

## Current stage (UART hello + M2–M9 + ADR-011 pillars)

- `Pl011` writer with TX-full wait and `\n` → `\r\n`; `try_recv` on `UARTFR.RXFE` / `UARTDR`
- `print!` / `println!` via `spin::Mutex`; fatal / unhandled / IRQ paths write the PL011 without the mutex
- After EL1 + VBAR, a bump frame allocator (`src/frame.rs`) and identity map (`src/paging.rs`) turn the MMU on ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md))
- First-fit `GlobalAlloc` (`src/heap.rs`) on a 64 KiB identity-mapped frame run ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)); `build-std` includes `alloc`
- Cooperative round-robin (`src/sched.rs`): idle on the linker thread stack, two heap-backed workers, AAPCS64 callee-saved yield ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md))
- `kernel_main` prints `Hello World!`, proves map/unmap (serial `paging: ok`), proves `Box`/`Vec` (serial `heap: ok`), proves two tasks (serial `sched: task a` / `sched: task b` / `sched: ok`), proves CNTPCT advances (serial `perf: cntpct delta=…`), observes one CNTP tick (serial `timer: tick`), polls one host-injected RX byte (serial `input: rx 0x41`), fires one healthy-stack `BRK #0` (serial `exception: sync BRK`), then the FR-07 nest probe (serial `exception: fatal nested`)
- `VBAR_EL1` vector table; kernel runs on `SP_EL0`; first-level current-EL sync (SP_EL0 bank) handles AArch64 `BRK` and returns; first-level IRQ handles GICv2 PPI 30; nested current-EL (SP_ELx bank) switches to the fatal stack ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md), [ADR-005](../03-adr/ADR-005-fatal-exception-stack.md), [ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md), [ADR-007](../03-adr/ADR-007-pl011-uart-rx.md))
- `cargo test` uses `#![feature(custom_test_frameworks)]` and `#[test_case]` (including VBAR, BRK, SPSel, stack ranges, GIC TYPER, CNTFRQ, timer tick, empty UART RX FIFO, MMU on, frames, map/unmap, heap `Box`/`Vec`, two-task yield, CNTPCT loop)
- QEMU exit is ARM **semihosting** `SYS_EXIT` / `hlt #0xf000` (`src/qemu.rs`), not `isa-debug-exit`. Needs `-semihosting` on the QEMU line (`scripts/qemu-aarch64.sh`).
- Host smoke: `scripts/qemu-smoke.sh` (hello + paging + heap + two-task sched + CNTPCT baseline + timer tick + injected UART RX + BRK + fatal nested strings + tests + `force-fail` must be non-zero)
- Docker: `Dockerfile` / `scripts/docker-smoke.sh` (linux/arm64-friendly; do not pin amd64)
- GHA: `.github/workflows/smoke.yml` (`ubuntu-24.04-arm` and `ubuntu-24.04`)

Source + local smoke were probed on 2026-09-08 (see the honesty ledger). GHA `smoke.yml` was green on that revision (`ubuntu-24.04` and `ubuntu-24.04-arm`). cts-ai Docker Desktop linux/arm64 `docker run --rm ctos-smoke` was Verified on 2026-09-09 after the LF / `cc` / ROM ratchets.

## Planned stages

| Stage | Domain work |
| --- | --- |
| Custom test framework | Landed (M2): `#[test_case]`, semihosting exit, UART |
| CPU exceptions | M3: `VBAR_EL1`, resumable `BRK`. M4: dedicated exception + fatal stacks (FR-07) — cloud `qemu-smoke` Verified (honesty ledger); GHA Unknown until a run URL |
| Hardware interrupts | M5: GICv2 + CNTP tick (FR-08) — cloud `qemu-smoke` + GHA Verified (honesty ledger). M6: PL011 UART RX (FR-08 input / ADR-007) |
| Paging | M7: EL1 identity map + bump frames (FR-09 / ADR-008) — probe status in the honesty ledger. DTB walk staged. |
| Heap | M8: first-fit `GlobalAlloc` on identity-mapped frames (FR-10 / ADR-009) — probe status in the honesty ledger. |
| Scheduler | M9: cooperative EL1 yield (FR-11 / ADR-010) — probe status in the honesty ledger. Not preemptive. |
| Pillars | [ADR-011](../03-adr/ADR-011-three-pillars.md): antifragility / security / performance. Threat-model stub + optional CNTPCT baseline. W^X / NX heap+stacks Planned (identity L1 RAM is executable). |

Each stage is one loop unit on the [roadmap](../04-roadmap/roadmap.md).

x86_64 remains a possible **future secondary** ISA. It is not a current tree.

## Harness mapping (short)

Hardware and QEMU are the **domain**. Docs, ADRs, and this architecture note are **shared understanding**. Guides, sensors, the one-PR loop, second-brain vaults, merge permissions, and the honesty ledger are the **outer harness**. Details: [formula.md](../framework/formula.md).
