# ctos — Functional & Non-Functional Requirements

Learning/research **x86_64 Rust** bare-metal kernel (not a general-purpose desktop OS).

**Status legend:** **Now** = hello-VGA / bootimage stage · **Next** = near roadmap · **Later** = aspirational.

Tracker is **GitHub** (`gh`). New IDs go through a GitHub issue plus an ADR/docs change — not chat.

## Scope

**In:** freestanding `no_std` Rust, bootloader → kernel, VGA/serial output, exceptions/interrupts, paging, heap, basic multitasking, reproducible build/QEMU probes, docs-first harness (honesty, antifragility, second brain, thin multi-agent).

**Out (for now):** userspace processes, POSIX, networking stack, GPU, SMP, real-hardware certification, secure-boot productization.

IDs below are **frozen**. Do not invent new FR/NFR IDs in chat; add via issue + ADR/docs change.

## Functional requirements

| ID | Requirement | Priority | Stage |
| --- | --- | --- | --- |
| **FR-01** | Kernel builds as a freestanding `no_std` / `no_main` binary for a custom `x86_64-*-none` target. | Must | Now |
| **FR-02** | Bootloader loads the kernel and transfers control to a stable `_start` entry. | Must | Now |
| **FR-03** | Kernel can write text to the VGA text buffer (`0xb8000`) via `print!` / `println!`. | Must | Now |
| **FR-04** | Panic handler prints a message (best-effort) and halts without unwinding. | Must | Now |
| **FR-05** | Developer can produce a bootable disk image (`bootimage`) and run it under QEMU. | Must | Now |
| **FR-06** | CPU exceptions are handled via an IDT (at least breakpoint / double-fault path). | Must | Next |
| **FR-07** | Double-fault uses a dedicated stack (TSS) so stack overflow doesn’t triple-fault silently. | Must | Next |
| **FR-08** | Hardware interrupts: timer and keyboard input via PIC/APIC. | Should | Next |
| **FR-09** | Kernel reads bootloader memory map and establishes paging / virtual memory. | Must | Next |
| **FR-10** | `GlobalAlloc` heap so `alloc` types (`Box`, `Vec`) work in kernel. | Should | Next |
| **FR-11** | Cooperative or simple round-robin task switching (threads or async tasks). | Should | Later |
| **FR-12** | Serial port output for headless/CI logs (in addition to or instead of VGA). | Should | Next |
| **FR-13** | Integration tests that boot in QEMU and exit with a deterministic success/fail code. | Must | Next |
| **FR-14** | Documented milestone path (vision → architecture → ADR → roadmap) stays ahead of code for each stage. | Must | Now |
| **FR-15** | Honesty ledger records each major capability as claim + probe + status (Verified / Unknown / Planned). | Must | Now |

## Non-functional requirements

| ID | Requirement | Priority | Stage |
| --- | --- | --- | --- |
| **NFR-01 Safety** | Prefer safe Rust; `unsafe` only at hardware/FFI boundaries, minimized and justified. | Must | Now |
| **NFR-02 Reproducibility** | Toolchain pinned (`rust-toolchain.toml`); target JSON + `.cargo/config.toml` checked in. | Must | Now |
| **NFR-03 Portability (dev)** | Build+QEMU path works on Windows/macOS/Linux (host tools); kernel target remains freestanding. | Must | Now |
| **NFR-04 CI / sensors** | PR gate runs at least build; later QEMU boot smoke with fail-closed exit codes. | Should | Next |
| **NFR-05 Antifragility** | Repeated failures become ratchets (Dockerfile pin, CI check, guide) — not README-only fixes. | Should | Next |
| **NFR-06 Honesty** | Status words in docs/PRs need a probe; unprobed stays Unknown; no self-merge as “verified.” | Must | Now |
| **NFR-07 Performance** | Early stages: correct boot + interrupt latency good enough for learning; no fake bench claims. | Could | Later |
| **NFR-08 Footprint** | Debug image size and boot time tracked once measurable; no premature optimization. | Could | Later |
| **NFR-09 Maintainability** | Clear module layout; ADRs for bootloader major-version or allocator choices. | Must | Now |
| **NFR-10 Security** | No secrets in repo; don’t claim “secure OS” until threat model + probes exist. | Must | Now |
| **NFR-11 Observability** | QEMU serial + explicit test exit codes are first telemetry; host Grafana out of scope. | Should | Next |
| **NFR-12 Multi-agent** | Thin roles (engineer / knowledge / coherence / MRC); implementer ≠ merge approver. | Should | Now |
| **NFR-13 Document-first** | Written milestone acceptance criteria match what code/CI actually prove. | Must | Now |
| **NFR-14 Second brain** | Session memory + daily briefs in git so agents inherit state without chat archaeology. | Should | Now |

## Acceptance sketch (Now)

- `cargo bootimage` succeeds with pinned nightly.
- QEMU shows the hello lines **or** honesty ledger says Unknown until probed.
- Panic path compiles and is reachable in principle.
- This FR/NFR file + honesty ledger live under `docs/`.

## Traceability

Map roadmap milestones and PRs to FR/NFR IDs in PR descriptions when touching behavior.
