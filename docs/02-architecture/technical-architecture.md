# Technical architecture

ctos is a `#![no_std]` `#![no_main]` binary. There is no Rust standard library and no OS underneath. The crate builds for a **custom target** (`aarch64-ctos.json`): `os: none`, panic abort, red zone off, static relocation, soft-float (no early SIMD/FP). See [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md). Guest samples (not “apps”): [apps-today.md](../framework/apps-today.md). Porting stance: [building-or-porting.md](../framework/building-or-porting.md).

## Boot path

1. `qemu-system-aarch64 -machine virt -cpu cortex-a57` (or `max`; `gic-version=3` is allowed).
2. `-kernel` loads the kernel ELF into virt RAM (linker base `0x40080000`).
3. Entry is `_start` (assembly in `src/main.rs`): set SP, zero BSS, call `kernel_main`.
4. Early output is the virt PL011 UART at `0x0900_0000` (`src/uart.rs`).
5. `kernel_main` installs `VBAR_EL1` (`src/exception.rs`, [ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)) after UART init, then the EL1 identity map plus a TTBR1 private page and a cloned TTBR1 RAM alias ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md), W^X split [ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md), stack guards [ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md), RO+NX [ADR-015](../03-adr/ADR-015-ro-nx-text-data.md), TTBR1 first cut [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md), EL1 fetch mile [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md), identity-tear first cut [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md), identity `.text` range tear [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md), live `.text` tear [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)), then the frame pool **after** MMU + `SCTLR.C` (pre-MMU `.bss` stores can vanish), then the first-fit heap ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)), then the cooperative scheduler ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)), then GICv2 + CNTP ([ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md)). After `Hello World!` it proves map/unmap, `Box`/`Vec`, two tasks, heap NX, linker-stack guards, the EL0 first mile + standing context, ASID isolation, the TTBR1 private page and high-VA EL1 fetch, torn identity `.text`, CNTPCT + IRQ-delta, several timer ticks, and polls PL011 RX ([ADR-007](../03-adr/ADR-007-pl011-uart-rx.md)).

There is no `bootloader` 0.9 crate and no VGA buffer. The x86_64 phil-opp path was deleted when this ADR landed.

`.cargo/config.toml` still needs `json-target-spec = true` on rustc 1.100 nightly. The AArch64 JSON is taken from `aarch64-unknown-none-softfloat` plus `os: none` / numeric widths. That nightly rejected the file until both `"abi": "softfloat"` and `"rustc-abi": "softfloat"` were set. Behavior is bare-metal AArch64, abort, `rust-lld`.

This architecture does **not** claim Raspberry Pi or other SoC support.

## Current stage (UART hello + M2–M9 + ADR-011 pillars)

- `Pl011` writer with TX-full wait and `\n` → `\r\n`; `try_recv` on `UARTFR.RXFE` / `UARTDR`
- `print!` / `println!` via `spin::Mutex`; fatal / unhandled / IRQ paths write the PL011 without the mutex
- After EL1 + VBAR, a bump frame allocator (`src/frame.rs`) and identity map (`src/paging.rs`) turn the MMU on ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)); RAM after `__kernel_end` is PXN ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)); 4 KiB holes sit under the linker stacks ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md))
- First-fit `GlobalAlloc` (`src/heap.rs`) on a 64 KiB identity-mapped frame run ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)); `build-std` includes `alloc`
- Cooperative round-robin (`src/sched.rs`): idle on the linker thread stack, two heap-backed workers, AAPCS64 callee-saved yield ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md))
- `kernel_main` prints `Hello World!`, proves map/unmap (serial `paging: ok`), proves `Box`/`Vec` (serial `heap: ok`), proves two tasks (serial `sched: task a` / `sched: task b` / `sched: ok`), proves heap NX (serial `wx: ok`), proves linker-stack guards (serial `guard: ok`), proves the EL0 first mile + standing context (serial `el0: ok` / `el0: standing` / `el0: restored`), proves the SVC ABI (serial `svc: yield` / `svc: user-hi` / `svc: uart` / `svc: exit` / `svc: ok`), proves ASID isolation (serial `asid: ok`), proves the TTBR1 private page and high-VA EL1 fetch (serial `ttbr1: ok` / `ttbr1: el1 exec` / `ttbr1: vbar`), proves the identity-tear first cut, `.text` range tear, vtable rewrite, and live `.text` tear (serial `ident: ok` / `ident: jump` / `ident: reloc` / `ident: range` / `ident: live` / `ident: text` / `ident: split` / `ident: fault` / `ident: high` / `ident: no el0`), proves CNTPCT advances (serial `perf: cntpct delta=…`), observes CNTP ticks (serial `timer: tick`) plus IRQ-to-handler deltas (serial `perf: irq-delta`), polls one host-injected RX byte (serial `input: rx 0x41`), fires one healthy-stack `BRK #0` (serial `exception: sync BRK`), then the FR-07 nest probe (serial `exception: fatal nested`)
- `VBAR_EL1` vector table (high alias after MMU, [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md)); kernel runs on `SP_EL0`; first-level current-EL sync (SP_EL0 bank) handles AArch64 `BRK`, heap NX, guard-page data aborts, and the identity-tear IABORT; first-level IRQ handles GICv2 PPI 30; lower-EL AArch64 sync handles the EL0 SVC / UXN IABORT / standing dual-SVC / public SVC ABI / TTBR1 DABORT / torn-page DABORT; nested current-EL (SP_ELx bank) switches to the fatal stack ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md), [ADR-005](../03-adr/ADR-005-fatal-exception-stack.md), [ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md), [ADR-007](../03-adr/ADR-007-pl011-uart-rx.md), [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md), [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md), [ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md), [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md), [ADR-019](../03-adr/ADR-019-identity-text-range-tear.md), [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md), [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md))
- `cargo test` uses `#![feature(custom_test_frameworks)]` and `#[test_case]` (including VBAR, BRK, SPSel, stack ranges, guards, GIC TYPER, CNTFRQ, timer tick, IRQ-delta samples, empty UART RX FIFO, MMU on, frames, map/unmap, heap `Box`/`Vec`, two-task yield, CNTPCT loop, W^X flags + execute-from-heap, EL0 first mile + standing enter/leave, SVC ABI yield/uart/exit + kernel-`.data` / TTBR1-alias reject, ASID isolation, TTBR1 private page + high-VA EL1 fetch, identity-tear first cut + `.text` range tear + vtable rewrite + live `.text` tear; `is_active()` is false at rest)
- QEMU exit is ARM **semihosting** `SYS_EXIT` / `hlt #0xf000` (`src/qemu.rs`), not `isa-debug-exit`. Needs `-semihosting` on the QEMU line (`scripts/qemu-aarch64.sh`).
- Host smoke: `scripts/qemu-smoke.sh` (hello + paging + heap + two-task sched + W^X + guards + EL0 first mile + standing + SVC ABI + ASID isolation + TTBR1 private page + high-VA EL1 fetch + identity-tear first cut + identity `.text` range tear + vtable rewrite + live `.text` tear + CNTPCT baseline + IRQ-delta + host ELF size + timer tick + injected UART RX + BRK + fatal nested strings + tests + `force-fail` must be non-zero)
- Docker: `Dockerfile` / `scripts/docker-smoke.sh` (linux/arm64-friendly; do not pin amd64)
- GHA: `.github/workflows/smoke.yml` (`ubuntu-24.04-arm` and `ubuntu-24.04`)

Source + local smoke were first probed on 2026-09-08 (see the honesty ledger). Current idle tip is `main` ≈ `e80dc93` (Merge PR #28 / ADR-020). cts-ai Docker Desktop linux/arm64 `./scripts/docker-smoke.sh` is **Verified** on that SHA (50 tests, `ident: reloc n=12`, live pages=37, force-fail ok). Earlier Docker Verified: `24d94e6` (ADR-019), `b0f0ee5` (#24–#26), `71ee15f` (layout L3). Keep the `b2bbb99` Failed row. GHA merge-commit [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) on `e80dc93` grepped the same class of markers. A Route 53 CNAME `ctos.artof.link` → `artofdream.github.io.` exists; **https://ctos.artof.link HTTPS is Verified** (2026-09-11 after #30). Do not claim “secure OS,” “the kernel moved,” or “EL0 isolated.”

## Planned stages

| Stage | Domain work |
| --- | --- |
| Custom test framework | Landed (M2): `#[test_case]`, semihosting exit, UART |
| CPU exceptions | M3: `VBAR_EL1`, resumable `BRK`. M4: dedicated exception + fatal stacks (FR-07) — cloud `qemu-smoke` Verified (honesty ledger); GHA Unknown until a run URL |
| Hardware interrupts | M5: GICv2 + CNTP tick (FR-08) — cloud `qemu-smoke` + GHA Verified (honesty ledger). M6: PL011 UART RX (FR-08 input / ADR-007) |
| Paging | M7: EL1 identity map + bump frames (FR-09 / ADR-008) — probe status in the honesty ledger. DTB walk staged. |
| Heap | M8: first-fit `GlobalAlloc` on identity-mapped frames (FR-10 / ADR-009) — probe status in the honesty ledger. |
| Scheduler | M9: cooperative EL1 yield (FR-11 / ADR-010) — probe status in the honesty ledger. Not preemptive. |
| Pillars | [ADR-011](../03-adr/ADR-011-three-pillars.md): antifragility / security / performance. Threat-model v1.9 ([security.md](../framework/security.md)). Heap NX ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)). Linker-stack guards ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md)). EL0 first mile + standing + ASID TLB mile; TTBR1 first cut ([ADR-016](../03-adr/ADR-016-ttbr1-private-page.md)); EL1 high-VA fetch ([ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md)); identity-tear first cut ([ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md)); identity `.text` range tear ([ADR-019](../03-adr/ADR-019-identity-text-range-tear.md)); live `.text` tear ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)); SVC ABI ([ADR-021](../03-adr/ADR-021-svc-syscall-abi.md)); umbrella isolation Planned ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)). App hosting Planned (Track A #31). |
| Filesystem | None today. **Planned** order (no FR ID): in-RAM memfs → virtio-blk → FAT or xv6-like. Stance: [filesystem.md](../framework/filesystem.md). |
| Host apps / containers | Gaps to Linux/shell/Python: [host-apps.md](../framework/host-apps.md). Guest container runtime is a **non-goal**. Host `docker-smoke` is unrelated. A1 is a kernel SVC ABI mile only. |
| Immutability | Scoped RO only ([immutability.md](../framework/immutability.md)). Absolute “immutable OS” is incompatible. Track A [#31](https://github.com/artofdream/ctos/issues/31) / Track B [#40](https://github.com/artofdream/ctos/issues/40). |

Each stage is one loop unit on the [roadmap](../04-roadmap/roadmap.md).

x86_64 remains a possible **future secondary** ISA. It is not a current tree.

## Harness mapping (short)

Hardware and QEMU are the **domain**. Docs, ADRs, and this architecture note are **shared understanding**. Guides, sensors, the one-PR loop, second-brain vaults, merge permissions, and the honesty ledger are the **outer harness**. Details: [formula.md](../framework/formula.md).
