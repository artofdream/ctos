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
| M9 | Cooperative scheduler | Two tasks observed to run | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on this PR still Unknown. |

M0–M9 are on `main` (M9 = merge of PR #15 / FR-11). This PR is **pillars only** ([ADR-011](../03-adr/ADR-011-three-pillars.md)): NFR-05 / NFR-07 / NFR-10 text, threat-model stub, optional one small CNTPCT ratchet. Not a second bring-up stack. Merge is still a human/MRC job (ADR-002).

## Pillars (post-M9)

Bring-up M0–M9 stays one loop unit each. After M9, work is grouped under the three pillars. A pillars *docs* PR may land **one** small code ratchet; do not stack W^X + a bench + a scheduler change.

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| P-SEC-1 | Threat-model stub (NFR-10) | Read [security.md](../framework/security.md); no “secure OS” claim | Stub in this PR (file read). Complete model **Planned**. |
| P-SEC-2 | W^X / NX heap + stacks | Page-table flags NX on the heap/stack region, or an execute-from-heap fault probe | **Planned.** Current identity L1 RAM block is executable ([ADR-008](../03-adr/ADR-008-identity-map-frame-allocator.md)). Needs an L2/L3 split. |
| P-PERF-1 | Baseline CNTPCT probe (NFR-07) | Serial `perf: cntpct` and/or `#[test_case]`; not a published bench | This PR if the small ratchet lands and a QEMU probe is recorded. Otherwise **Planned**. |
| P-SEC-3 | EL0 isolation | Later ADR + probe that a lower-EL map cannot execute kernel data | **Planned** (vision Out until then). |

Hub: [pillars.md](../framework/pillars.md).
