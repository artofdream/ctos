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
| M9 | Cooperative scheduler | Two tasks observed to run | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` (see honesty ledger). |

M0–M9 are on `main` (M9 = merge of PR #15 / FR-11). ADR-011 / #17 through ASID isolation #23 are on `main`. This PR is standing EL0 + TTBR1 first cut (ADR-013 / ADR-016). Merge is still a human/MRC job (ADR-002).

## Pillars (post-M9)

Bring-up M0–M9 stays one loop unit each. After M9, work is grouped under the three pillars.

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| P-SEC-1 | Threat-model v1.1 slice (NFR-10) | Read [security.md](../framework/security.md); no “secure OS” claim | **Verified** (file + review). v1.1 adds linker stacks, guards, EL0 first mile, remaining W^X gaps. |
| P-SEC-2 | W^X / NX heap + coop stacks | Page-table PXN on heap + caught execute-from-heap IABORT (`wx: ok`) | **Verified:** 2026-09-10 cloud `qemu-smoke` (see honesty ledger). Map: [ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md). Linker stack **pages** still X. |
| P-SEC-2b | Linker-stack guard pages | Unmapped 4 KiB holes; store faults (`guard: ok`) | **Verified:** 2026-09-10 cloud `qemu-smoke` (see honesty ledger). [ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md). Not kernel W^X. |
| P-PERF-1 | Baseline CNTPCT probe (NFR-07) | Serial `perf: cntpct` and/or `#[test_case]`; not a published bench | Verified: 2026-09-09 cloud `qemu-smoke` + GHA (see honesty ledger). |
| P-PERF-2 | IRQ-to-handler CNTPCT delta | Serial `perf: irq-delta` + samples `max >= min`; not a latency budget | **Verified:** 2026-09-10 cloud `qemu-smoke` (see honesty ledger). |
| P-PERF-3 | Host debug ELF size (NFR-08) | Host marker `perf: elf-size bytes=<n>`; not a budget | **Verified:** 2026-09-10 cloud `qemu-smoke` host print (see honesty ledger). |
| P-SEC-3 | EL0 first mile | `el0: svc` + `el0: nx kernel` + `el0: ok` | **First mile Verified:** 2026-09-10 cloud `qemu-smoke` (see honesty ledger). |
| P-SEC-2c | RO+NX text/data (ADR-015) | `ro: nx data` + `ro: write fault` + `ro: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Identity image W^X on virt; not “secure OS.” |
| P-PERF-4 | Boot-to-ready CNTPCT (NFR-08) | `perf: boot-delta ticks=<n>`; not a budget | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). |
| P-SEC-3b | User TTBR0 + EL0 cannot read kernel `.data` | `el0: no kernel read`; user table omits `.data` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Isolation / PAN stay **Planned**. |
| P-SEC-3c | ASID-tagged TLB isolation | `asid: dual` + `asid: conflict` + `asid: ok`; no `TLBI VMALLE1` on the switch | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Umbrella isolation / PAN stay **Planned**. |
| P-SEC-3d | Standing EL0 context | `el0: standing` + `el0: restored`; `is_active()` true only while standing | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Not POSIX. Lower-EL IRQ still parks. |
| P-SEC-3e | TTBR1 kernel-private page (ADR-016 first cut) | `ttbr1: el1` + `ttbr1: no el0` + `ttbr1: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Identity teardown / full higher-half stay **Planned**. |

Hub: [pillars.md](../framework/pillars.md).
