# Roadmap

Each row is **one loop unit**: one milestone, one branch, one GitHub PR. Do not stack a later stage onto an unclosed earlier PR. Cite frozen [FR/NFR IDs](../02-requirements/fr-nfr.md) in the PR when touching behavior; do not invent new IDs in chat.

Primary ISA is AArch64 ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). M0/M1 probes are UART + `qemu-system-aarch64`, not VGA `0xb8000`.

| ID | Milestone | Probe that closes it | Status |
| --- | --- | --- | --- |
| M0 | Kernel tree + harness scaffold on GitHub | Files on the default-target PR; ledger started | In harness PRs #2/#3 (source). Merge is a human/MRC job. |
| M1 | QEMU `virt` boots UART "Hello World!" | `qemu-system-aarch64` serial shows the string; see honesty ledger | Verified in the 2026-09-08 cloud probe (not CI). x86 VGA probe is historical only. |
| M2 | Integration test harness | QEMU virt ARM semihosting exit + `#[test_case]`; `scripts/qemu-smoke.sh` fail-closed | Verified in the 2026-09-08 cloud probe (not CI). Sponsor stacked this on the ISA PR. |
| M3 | Exception vectors + breakpoint | Test or QEMU serial proof the handler runs | In this PR (probe in honesty ledger). |
| M4 | Fatal exception stack | Fatal path does not silently lock the VM | Planned |
| M5 | Hardware interrupts (GIC + timer) | Timer tick observable (serial or test) | Planned |
| M6 | Input | UART or virtio input path prints or tests a key | Planned |
| M7 | Paging + frame allocator | Map/unmap or allocator test | Planned |
| M8 | Heap (`alloc`) | Box/vec smoke on the heap | Planned |
| M9 | Cooperative scheduler | Two tasks observed to run | Planned |

M0–M2 are on `main` (harness + AArch64 UART + semihosting tests). This PR is **M3 only** (VBAR_EL1 + BRK). M4+ stay one PR each. Merge is still a human/MRC job (ADR-002).
