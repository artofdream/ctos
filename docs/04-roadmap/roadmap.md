# Roadmap

Each row is **one loop unit**: one milestone, one branch, one GitHub PR. Do not stack a later stage onto an unclosed earlier PR. Cite frozen [FR/NFR IDs](../02-requirements/fr-nfr.md) in the PR when touching behavior; do not invent new IDs in chat.

Primary ISA is AArch64 ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). M0/M1 probes are UART + `qemu-system-aarch64`, not VGA `0xb8000`.

| ID | Milestone | Probe that closes it | Status |
| --- | --- | --- | --- |
| M0 | Kernel tree + harness scaffold on GitHub | Files on the default-target PR; ledger started | In harness PRs #2/#3 (source). Merge is a human/MRC job. |
| M1 | QEMU `virt` boots UART "Hello World!" | `qemu-system-aarch64` serial shows the string; see honesty ledger | Verified in the 2026-09-08 cloud probe (not CI). x86 VGA probe is historical only. |
| M2 | Integration test harness | QEMU virt ARM semihosting exit + `#[test_case]`; `scripts/qemu-smoke.sh` fail-closed | Verified in the 2026-09-08 cloud probe (not CI). Sponsor stacked this on the ISA PR. |
| M3 | Exception vectors + breakpoint | Test or QEMU serial proof the handler runs | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on this PR (see honesty ledger). |
| M4 | Fatal exception stack | Fatal path does not silently lock the VM | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on this PR still Unknown. |
| M5 | Hardware interrupts (GIC + timer) | Timer tick observable (serial or test) | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on this PR (see honesty ledger). |
| M6 | Input | Injected PL011 RX byte observable on serial (or test) | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on this PR still Unknown. |
| M7 | Paging + frame allocator | Map/unmap or allocator test | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on this PR still Unknown. |
| M8 | Heap (`alloc`) | Box/vec smoke on the heap | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on this PR (see honesty ledger). |
| M9 | Cooperative scheduler | Two tasks observed to run | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on this PR (see honesty ledger). |

M0–M8 are on `main` (M8 = merge of PR #14 / FR-10). This PR is **M9 only** (cooperative round-robin on EL1, FR-11 / [ADR-010](../03-adr/ADR-010-cooperative-rr-el1.md)). Preemption is a later ADR. Merge is still a human/MRC job (ADR-002).
