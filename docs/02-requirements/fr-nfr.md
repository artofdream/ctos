# ctos — Functional & Non-Functional Requirements

Learning/research **AArch64 (arm64) Rust** bare-metal kernel (not a general-purpose desktop OS).

**ISA revision:** [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md) (2026-09-08) replaced x86_64 / VGA / `bootimage` wording with AArch64 / UART / QEMU `virt`. IDs are unchanged. Sponsor authorized the text revision; do not mint new FR/NFR IDs here. The sponsor brief called this “ADR-002”; that number was already the identity-split ADR on the harness tip.

**FR-06 stage note (2026-09-09):** Stage moved Next → Now when M3 landed `VBAR_EL1` + resumable `BRK` ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)). ID unchanged.

**FR-07 stage note (2026-09-09):** Stage moved Next → Now when M4 landed the dedicated exception / fatal stacks ([ADR-005](../03-adr/ADR-005-fatal-exception-stack.md)). ID unchanged.

**FR-08 stage note (2026-09-09):** Stage moved Next → Now when M5 landed GICv2 + the EL1 physical timer ([ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md)). ID unchanged. FR-08 “later input” is M6 / [ADR-007](../03-adr/ADR-007-pl011-uart-rx.md) (PL011 RX on virt). No new FR ID.

**FR-09 stage note (2026-09-09):** Stage moved Next → Now when M7 landed the EL1 identity map + bump frame allocator ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)). ID unchanged. DTB parse remains staged (Unknown). Heap is M8 / FR-10.

**FR-10 stage note (2026-09-09):** Stage moved Next → Now when M8 landed `GlobalAlloc` on a first-fit heap backed by identity-mapped frames ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)). ID unchanged. Growing / slab heaps remain later.

**FR-11 stage note (2026-09-09):** Stage moved Later → Now when M9 landed cooperative round-robin yield on EL1 ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). ID unchanged. Preemption / SMP / EL0 remain later.

**NFR pillars note (2026-09-09):** [ADR-011](../03-adr/ADR-011-three-pillars.md) revises **NFR-05**, **NFR-07**, and **NFR-10** text in place (antifragility, performance, security as first-class pillars). IDs unchanged. Do not mint NFR-15+.

**NFR-10 W^X note (2026-09-10):** [ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md) revises NFR-10 text in place for the heap / coop-stack NX cut. [ADR-013](../03-adr/ADR-013-el0-isolation-direction.md) records EL0 direction; the first mile is enter/return + UXN fetch, not isolation. [ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md) adds unmapped linker-stack guards. IDs unchanged.

**NFR-10 / NFR-08 note (2026-09-11):** [ADR-015](../03-adr/ADR-015-ro-nx-text-data.md) revises NFR-10 text in place for the RO+NX text/data cut (`SCTLR.WXN` on). ADR-013 adds a user-TTBR0 read mile, an ASID-tagged TLB mile, and a standing EL0 context; [ADR-016](../03-adr/ADR-016-ttbr1-private-page.md) adds a TTBR1 private-page first cut. The umbrella isolation row stays Planned (PAN + full higher-half still missing). NFR-08 boot-to-ready CNTPCT is a measurement, not a budget. IDs unchanged.

**Status legend:** **Now** = hello-UART / QEMU `virt` / smoke sensors · **Next** = near roadmap · **Later** = aspirational.

Tracker is **GitHub** (`gh`). New IDs go through a GitHub issue plus an ADR/docs change — not chat.

## Scope

**In:** freestanding `no_std` Rust, QEMU `-kernel` → `_start`, UART/earlycon output, exceptions/interrupts, paging, heap, basic multitasking, reproducible build/QEMU probes, docs-first harness (honesty, three pillars — antifragility / security / performance — second brain, thin multi-agent).

**Out (for now):** userspace processes, POSIX, networking stack, GPU, SMP, real-hardware certification, secure-boot productization, Raspberry Pi or other board bring-up (unprobed), x86_64 as a primary path.

IDs below are **frozen**. Do not invent new FR/NFR IDs in chat; add via issue + ADR/docs change.

## Functional requirements

| ID | Requirement | Priority | Stage |
| --- | --- | --- | --- |
| **FR-01** | Kernel builds as a freestanding `no_std` / `no_main` binary for a custom `aarch64-*-none` target. | Must | Now |
| **FR-02** | QEMU (`-kernel` on `virt`) loads the kernel ELF and transfers control to a stable `_start` entry. | Must | Now |
| **FR-03** | Kernel can write text to the virt PL011 UART (MMIO, or later earlycon) via `print!` / `println!`. Not VGA `0xb8000`. | Must | Now |
| **FR-04** | Panic handler prints a message (best-effort) and halts without unwinding. | Must | Now |
| **FR-05** | Developer can produce a runnable AArch64 kernel ELF and boot it under `qemu-system-aarch64` (`-machine virt`). | Must | Now |
| **FR-06** | Synchronous exceptions are handled via VBAR_EL1 vectors (at least a breakpoint / fault path). | Must | Now |
| **FR-07** | A fatal exception uses a dedicated stack so overflow does not silently lock the VM. | Must | Now |
| **FR-08** | Hardware interrupts: timer via the virt GIC (M5). Input on QEMU virt is PL011 UART RX (M6). The x86-era keyboard wording is this UART path; virtio-keyboard remains later. | Should | Now |
| **FR-09** | Kernel reads the firmware/QEMU memory map (DTB when probed) and establishes paging / virtual memory. M7: virt RAM convention + linker `__kernel_end` + EL1 identity map ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)). DTB walk is staged. | Must | Now |
| **FR-10** | `GlobalAlloc` heap so `alloc` types (`Box`, `Vec`) work in kernel. M8: first-fit list on a 64 KiB identity-mapped frame run ([ADR-009](../03-adr/ADR-009-first-fit-heap.md)). | Should | Now |
| **FR-11** | Cooperative or simple round-robin task switching (threads or async tasks). M9: EL1 yield of AAPCS64 callee-saved GPRs; two heap-backed workers ([ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). | Should | Now |
| **FR-12** | Serial remains usable for headless/CI logs (PL011 or extra earlycon), including later test-exit telemetry. | Should | Next |
| **FR-13** | Integration tests that boot in QEMU `virt` and exit with a deterministic success/fail code (ARM semihosting SYS_EXIT, not x86 isa-debug-exit). | Must | Now |
| **FR-14** | Documented milestone path (vision → architecture → ADR → roadmap) stays ahead of code for each stage. | Must | Now |
| **FR-15** | Honesty ledger records each major capability as claim + probe + status (Verified / Unknown / Planned). | Must | Now |

## Non-functional requirements

| ID | Requirement | Priority | Stage |
| --- | --- | --- | --- |
| **NFR-01 Safety** | Prefer safe Rust; `unsafe` only at hardware/FFI boundaries, minimized and justified. | Must | Now |
| **NFR-02 Reproducibility** | Toolchain pinned (`rust-toolchain.toml`); target JSON + `.cargo/config.toml` checked in. | Must | Now |
| **NFR-03 Portability (dev)** | Build+QEMU path works on Windows/macOS/Linux (host tools); kernel target remains freestanding AArch64. | Must | Now |
| **NFR-04 CI / sensors** | PR gate runs `scripts/qemu-smoke.sh` (build + UART hello string + `cargo test` exit codes). GitHub Actions + optional Docker. | Should | Now |
| **NFR-05 Antifragility** | First-class pillar ([ADR-011](../03-adr/ADR-011-three-pillars.md)). Repeated failures become ratchets (`#[test_case]`, `scripts/qemu-smoke.sh` grep, Dockerfile pin, GHA) — not README-only fixes. Fail-closed sensors stay on the strongest layer. | Must | Now |
| **NFR-06 Honesty** | Status words in docs/PRs need a probe; unprobed stays Unknown; no self-merge as “verified.” | Must | Now |
| **NFR-07 Performance** | First-class pillar ([ADR-011](../03-adr/ADR-011-three-pillars.md)). Performance claims need a measurable probe (CNTPCT delta around a known path; IRQ-to-handler CNTPCT−CVAL min/max/spread). No fake benches and no “faster than X.” Optimize only after a probe shows a cost. Not a hard latency budget. | Should | Now |
| **NFR-08 Footprint** | Debug image size and boot time tracked once measurable; no premature optimization. | Could | Later |
| **NFR-09 Maintainability** | Clear module layout; ADRs for boot-path, console, or allocator choices. | Must | Now |
| **NFR-10 Security** | First-class pillar ([ADR-011](../03-adr/ADR-011-three-pillars.md)). No secrets in repo. Do not claim “secure OS” / “hardened” / “the kernel is W^X” without a written threat model **and** a probe. Prefer minimize `unsafe` (NFR-01). Heap and heap-backed cooperative stacks are PXN ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)). Linker-stack downward overflow hits an unmapped 4 KiB guard ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md)). `.text`/`.rodata` are RO+X and `.data`/`.bss`/live linker stacks are RW+NX; `SCTLR.WXN` is on ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). That scoped image cut is not a “secure OS” sentence. Device MMIO XN. Least privilege on IRQ paths. EL0 first mile (enter/return + cannot execute kernel data), the user-TTBR0 read mile, the ASID-tagged TLB mile, and the standing EL0 context are probes ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)). TTBR1 private-page first cut is a probe ([ADR-016](../03-adr/ADR-016-ttbr1-private-page.md)); identity teardown stays Planned. Umbrella isolation stays Planned. Do not claim PAN on `-cpu cortex-a57`. | Must | Now |
| **NFR-11 Observability** | QEMU UART + explicit test exit codes are first telemetry; host Grafana out of scope. | Should | Next |
| **NFR-12 Multi-agent** | Thin roles (engineer / knowledge / coherence / MRC); implementer ≠ merge approver. | Should | Now |
| **NFR-13 Document-first** | Written milestone acceptance criteria match what code/CI actually prove. | Must | Now |
| **NFR-14 Second brain** | Session memory + daily briefs in git so agents inherit state without chat archaeology. | Should | Now |

## Acceptance sketch (Now)

- `cargo build` succeeds with pinned nightly for `aarch64-ctos.json`.
- `qemu-system-aarch64 -machine virt` shows the hello line on serial **or** honesty ledger says Unknown until probed.
- `scripts/qemu-smoke.sh` fails closed if the hello string is missing or `cargo test` does not exit 0.
- `cargo test` uses ARM semihosting so QEMU exits 0 on pass and 1 on panic (`force-fail`).
- Current-EL `BRK` is handled via `VBAR_EL1` (serial string and/or `#[test_case]`) **or** the honesty ledger says Unknown until probed.
- Nested / fatal exception prints a serial marker from a dedicated stack (FR-07) **or** the honesty ledger says Unknown until probed.
- A timer tick is observable on serial and/or `#[test_case]` via the virt GIC (FR-08 / M5) **or** the honesty ledger says Unknown until probed.
- A received PL011 byte is observable on serial (FR-08 input / M6) **or** the honesty ledger says Unknown until probed.
- MMU-on + allocate-frame / map-unmap is observable on serial and/or `#[test_case]` (FR-09 / M7) **or** the honesty ledger says Unknown until probed.
- `Box` / `Vec` on the kernel heap is observable on serial and/or `#[test_case]` (FR-10 / M8) **or** the honesty ledger says Unknown until probed.
- Two cooperative tasks are observable on serial and/or `#[test_case]` (FR-11 / M9) **or** the honesty ledger says Unknown until probed.
- A CNTPCT baseline delta is observable on serial and/or `#[test_case]` (NFR-07) **or** the honesty ledger says Unknown / Planned until probed.
- An IRQ-to-handler CNTPCT delta (min/max/spread) is observable on serial and/or `#[test_case]` (NFR-07) **or** the honesty ledger says Unknown until probed.
- Threat-model v1.1 exists under `docs/framework/security.md`; “secure OS” stays unclaimed (NFR-10).
- Heap NX / execute-from-heap caught is observable on serial and/or `#[test_case]` (NFR-10 / ADR-012) **or** the honesty ledger says Unknown until probed.
- Linker-stack guard store faults observably (NFR-10 / ADR-014) **or** the honesty ledger says Unknown until probed.
- EL0 first mile (SVC return and/or UXN IABORT) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-013) **or** the honesty ledger says Unknown / Planned until probed.
- Host debug ELF size is printed by `scripts/qemu-smoke.sh` (NFR-08) **or** the honesty ledger says Unknown until probed.
- Boot-to-ready CNTPCT (`perf: boot-delta`) is observable on serial and/or `#[test_case]` (NFR-08) **or** the honesty ledger says Unknown until probed.
- RO+NX text/data (execute-from-`.data` and write-to-RO-text) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-015) **or** the honesty ledger says Unknown until probed.
- EL0 cannot-read-kernel-data (user TTBR0) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-013) **or** the honesty ledger says Unknown / Planned until probed.
- ASID isolation (dual ASID without `TLBI VMALLE1` + conflict/stale-entry probe) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-013) **or** the honesty ledger says Unknown / Planned until probed.
- Standing EL0 (`el0: standing` / `el0: restored`, `is_active()` flips) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-013) **or** the honesty ledger says Unknown / Planned until probed.
- TTBR1 private page (`ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok`) is observable on serial and/or `#[test_case]` (NFR-10 / ADR-016) **or** the honesty ledger says Unknown / Planned until probed. Full higher-half teardown stays Planned. Umbrella isolation stays Planned. Do not claim PAN.
- Panic path compiles and is reachable in principle.
- This FR/NFR file + honesty ledger live under `docs/`.

## Traceability

Map roadmap milestones and PRs to FR/NFR IDs in PR descriptions when touching behavior.
