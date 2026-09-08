# Technical architecture

ctos is a `#![no_std]` `#![no_main]` binary. There is no Rust standard library and no OS underneath. The crate builds for a **custom target** (`x86_64-ctos.json`): `os: none`, panic abort, red zone off, SIMD off, soft-float.

## Boot path

1. Firmware (BIOS path via the `bootloader` **0.9** crate).
2. `bootimage` wraps the kernel ELF into a disk image.
3. Entry is `_start` in `src/main.rs`.
4. Early output is VGA text at `0xb8000` (`src/vga_buffer.rs`).

Stay on bootloader 0.9 / volatile 0.2 / spin 0.5. Do not migrate to bootloader 0.10 unless a dedicated ADR says so.

The custom target started from the tutorial-era JSON. A 2026-09-08 `rustc` 1.100 nightly probe rejected that file until we ratcheted: numeric `target-pointer-width` / `target-c-int-width`, LLVM `data-layout` with p270/p271/p272 and i128, and `rustc-abi: softfloat`. `.cargo/config.toml` also needs `json-target-spec = true` on that nightly. Behavior is still bare-metal x86_64, abort, no red zone, soft-float, `rust-lld`.

## Current stage (VGA)

- `Color` / `ColorCode` / `ScreenChar` / `Buffer`
- `Writer` with wrap + scroll
- `print!` / `println!` via `lazy_static` + `spin::Mutex`
- Yellow on black, matching the common tutorial writer

Source is present. QEMU showing "Hello World!" was probed on 2026-09-08 (see the honesty ledger). That probe is not CI.

## Planned stages (phil-opp order)

| Stage | Domain work |
| --- | --- |
| Custom test framework | `#[test_case]`, QEMU isa-debug-exit, serial |
| CPU exceptions | IDT, breakpoint, double-fault IST |
| Hardware interrupts | PIC, timer, keyboard |
| Paging | page tables, frame allocator |
| Heap | `alloc`, a simple allocator |
| Scheduler | cooperative or round-robin tasks |

Each stage is one loop unit on the [roadmap](../04-roadmap/roadmap.md).

## Harness mapping (short)

Hardware and QEMU are the **domain**. Docs, ADRs, and this architecture note are **shared understanding**. Guides, sensors, the one-PR loop, second-brain vaults, merge permissions, and the honesty ledger are the **outer harness**. Details: [formula.md](../framework/formula.md).
