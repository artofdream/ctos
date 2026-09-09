# ctos — Functional & Non-Functional Requirements

Learning/research **AArch64 (arm64) Rust** bare-metal kernel (not a general-purpose desktop OS).

**ISA revision:** [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md) (2026-09-08) replaced x86_64 / VGA / `bootimage` wording with AArch64 / UART / QEMU `virt`. IDs are unchanged. Sponsor authorized the text revision; do not mint new FR/NFR IDs here. The sponsor brief called this “ADR-002”; that number was already the identity-split ADR on the harness tip.

**FR-06 stage note (2026-09-09):** Stage moved Next → Now when M3 landed `VBAR_EL1` + resumable `BRK` ([ADR-004](../03-adr/ADR-004-el1-vbar-brk.md)). ID unchanged.

**FR-07 stage note (2026-09-09):** Stage moved Next → Now when M4 landed the dedicated exception / fatal stacks ([ADR-005](../03-adr/ADR-005-fatal-exception-stack.md)). ID unchanged.

**FR-08 stage note (2026-09-09):** Stage moved Next → Now when M5 landed GICv2 + the EL1 physical timer ([ADR-006](../03-adr/ADR-006-gicv2-generic-timer.md)). ID unchanged. FR-08 “later input” is M6 / [ADR-007](../03-adr/ADR-007-pl011-uart-rx.md) (PL011 RX on virt). No new FR ID.

**Status legend:** **Now** = hello-UART / QEMU `virt` / smoke sensors · **Next** = near roadmap · **Later** = aspirational.

Tracker is **GitHub** (`gh`). New IDs go through a GitHub issue plus an ADR/docs change — not chat.

## Scope

**In:** freestanding `no_std` Rust, QEMU `-kernel` → `_start`, UART/earlycon output, exceptions/interrupts, paging, heap, basic multitasking, reproducible build/QEMU probes, docs-first harness (honesty, antifragility, second brain, thin multi-agent).

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
| **FR-09** | Kernel reads the firmware/QEMU memory map (DTB when probed) and establishes paging / virtual memory. | Must | Next |
| **FR-10** | `GlobalAlloc` heap so `alloc` types (`Box`, `Vec`) work in kernel. | Should | Next |
| **FR-11** | Cooperative or simple round-robin task switching (threads or async tasks). | Should | Later |
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
| **NFR-05 Antifragility** | Repeated failures become ratchets (Dockerfile pin, CI check, guide) — not README-only fixes. | Should | Next |
| **NFR-06 Honesty** | Status words in docs/PRs need a probe; unprobed stays Unknown; no self-merge as “verified.” | Must | Now |
| **NFR-07 Performance** | Early stages: correct boot + interrupt latency good enough for learning; no fake bench claims. | Could | Later |
| **NFR-08 Footprint** | Debug image size and boot time tracked once measurable; no premature optimization. | Could | Later |
| **NFR-09 Maintainability** | Clear module layout; ADRs for boot-path, console, or allocator choices. | Must | Now |
| **NFR-10 Security** | No secrets in repo; don’t claim “secure OS” until threat model + probes exist. | Must | Now |
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
- Panic path compiles and is reachable in principle.
- This FR/NFR file + honesty ledger live under `docs/`.

## Traceability

Map roadmap milestones and PRs to FR/NFR IDs in PR descriptions when touching behavior.
