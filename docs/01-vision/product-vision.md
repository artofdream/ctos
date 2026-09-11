# Product vision

## What ctos is

ctos is a **learning and research** bare-metal OS kernel written in Rust for **AArch64** (arm64 primary; [ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). The near-term product is a QEMU `virt` guest that owns the machine after `-kernel` load: UART text, then exceptions, paging, a heap, and a tiny scheduler — in that order.

It exists so we can study kernel mechanics with a document-first harness: claims stay honest, failures become sensors, and agents do not merge their own work. After M9 that harness sits on **three first-class pillars** — antifragility, security, and performance ([ADR-011](../03-adr/ADR-011-three-pillars.md)). Security and speed are not afterthoughts; they still need probes before anyone writes “secure” or “fast.”

## What ctos is not

- Not a production operating system, desktop, or app runtime.
- Not a Linux distro, not POSIX, not a container host.
- Not a florist / commerce platform and not a clone of any shop case study.
- Not a Raspberry Pi (or other board) port until a board probe exists.
- Not a claim that QEMU boot, CI, hardware bring-up, a “secure OS,” or a published bench is finished until a probe says so.
- Not an x86_64-primary kernel. x86_64 may become a secondary target later; it is not implemented now.

## Success (current horizon)

A new session can read the vision, [overview](../framework/overview.md), architecture, honesty ledger, and latest daily brief, then take **one** roadmap milestone to a GitHub PR without inventing status.
