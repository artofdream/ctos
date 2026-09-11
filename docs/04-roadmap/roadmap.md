# Roadmap

Each row is **one loop unit**: one milestone, one branch, one GitHub PR. Do not stack a later stage onto an unclosed earlier PR. Cite frozen [FR/NFR IDs](../02-requirements/fr-nfr.md) in the PR when touching behavior; do not invent new IDs in chat.

Primary ISA is AArch64 ([ADR-003](../03-adr/ADR-003-primary-isa-aarch64.md)). M0/M1 probes are UART + `qemu-system-aarch64`, not VGA `0xb8000`.

| ID | Milestone | Probe that closes it | Status |
| --- | --- | --- | --- |
| M0 | Kernel tree + harness scaffold on GitHub | Files on the default-target PR; ledger started | In harness PRs #2/#3 (source). Merge is a human/MRC job. |
| M1 | QEMU `virt` boots UART "Hello World!" | `qemu-system-aarch64` serial shows the string; see honesty ledger | Verified in the 2026-09-08 cloud probe (not CI). x86 VGA probe is historical only. |
| M2 | Integration test harness | QEMU virt ARM semihosting exit + `#[test_case]`; `scripts/qemu-smoke.sh` fail-closed | Verified in the 2026-09-08 cloud probe (not CI). Sponsor stacked this on the ISA PR. |
| M3 | Exception vectors + breakpoint | Test or QEMU serial proof the handler runs | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on the M3 PR (see honesty ledger). |
| M4 | Fatal exception stack | Fatal path does not silently lock the VM | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on the M4 PR branch still Unknown. |
| M5 | Hardware interrupts (GIC + timer) | Timer tick observable (serial or test) | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on the M5 PR (see honesty ledger). |
| M6 | Input | Injected PL011 RX byte observable on serial (or test) | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on the M6 PR branch still Unknown. |
| M7 | Paging + frame allocator | Map/unmap or allocator test | Verified: 2026-09-09 cloud `qemu-smoke` (see honesty ledger). GHA on the M7 PR branch still Unknown. |
| M8 | Heap (`alloc`) | Box/vec smoke on the heap | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` on the M8 PR (see honesty ledger). |
| M9 | Cooperative scheduler | Two tasks observed to run | Verified: 2026-09-09 cloud `qemu-smoke` + GHA `smoke.yml` (see honesty ledger). |

M0–M9 are on `main` (M9 = merge of PR #15 / FR-11). Pillar work through ADR-020 (#17–#28) plus docs #30/#51/#29 are on `main`. This PR is Track A / A1 ([issue #32](https://github.com/artofdream/ctos/issues/32), parent [issue #31](https://github.com/artofdream/ctos/issues/31)): ADR-021 SVC ABI. Merge is still a human/MRC job (ADR-002).

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
| P-SEC-3e | TTBR1 kernel-private page (ADR-016 first cut) | `ttbr1: el1` + `ttbr1: no el0` + `ttbr1: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). |
| P-SEC-3f | EL1 fetch from TTBR1 high VA (ADR-017) | `ttbr1: el1 exec` + `ttbr1: vbar` + existing `ttbr1: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Identity boot stub stays. |
| P-SEC-3g | Identity-tear first cut (ADR-018) | `ident: split` + `ident: fault` + `ident: high` + `ident: no el0` + `ident: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Full identity teardown / PAN / umbrella isolation stay **Planned**. Do not claim “the kernel moved.” |
| P-SEC-3h | High-VA jump + 16 KiB identity text range (ADR-019) | `ident: jump` + `ident: range` + `ident: text` + existing `ident: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` + cts-ai Docker on `24d94e6` (see honesty ledger). Do not claim “the kernel moved.” |
| P-SEC-3i | High-VA vtable rewrite + live identity `.text` tear (ADR-020) | `ident: reloc` + `ident: live` + existing `ident: ok`; `println!` after the tear | **Verified:** 2026-09-11 cloud `qemu-smoke` + cts-ai Docker on `e80dc93` + GHA merge-commit [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) (see honesty ledger). `.rodata` / `.data` / heap stay. Do not claim “the kernel moved.” |
| P-SEC-3j | Identity `.rodata` / `.data` / heap tear | Those identity ranges unmapped; accesses proven high-only | **Planned.** After live `.text` (ADR-020), not instead of it. |
| P-SEC-3k | PAN on virt `cortex-a57` | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1-vs-EL0 access fault | **Planned.** ARMv8.0 `cortex-a57`. Do not switch `-cpu` silently. |
| P-SEC-3l | Umbrella EL0 isolation | Standing + PAN + full TTBR1 / identity teardown | **Planned.** Specific miles (P-SEC-3…P-SEC-3k) are not this row. Do not claim “EL0 isolated.” |

Hub: [pillars.md](../framework/pillars.md). ABI contract: [syscall.md](../framework/syscall.md).

## Track A — freestanding app hosting ([#31](https://github.com/artofdream/ctos/issues/31))

A1 is the SVC ABI mile only. **Track A stays incomplete** after A1 (A2–A9 Planned). Not Linux containers. Not glibc. One child issue → one PR. Do not round A1 Verified up to “app hosting is done.”

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| A1 | Stable SVC ABI + docs ([#32](https://github.com/artofdream/ctos/issues/32), ADR-021) | `svc: yield` + `svc: user-hi` + `svc: uart` + `svc: exit` + `svc: ok`; `#[test_case]` | **Unknown** on rebase onto `aa46219` until `qemu-smoke`. App hosting stays **Planned**. |
| A2 | Freestanding CRT / `libctos` | crate + probes wrapping exit / uart_write / yield | **Planned** |
| A3 | ELF (or raw image) loader into user TTBR0 | loaded image runs at EL0 | **Planned** |
| A4 | Standing EL0 as normal mode | not only a smoke probe | **Planned** |
| A5 | Isolation completion | remaining identity tear / PAN only with ADR | **Planned** |
| A6 | Thin VFS + memfs | path walk + read probe | **Planned** |
| A7 | virtio-blk + FAT or xv6-like FS | block + fs probe | **Planned** |
| A8–A9 | Sample in-tree coop UART / standing EL0 app | documented sample serial | **Planned** |

Filesystem work is **Planned** and is not a row above (A6–A7). Intended order (one PR each, after a VFS ADR): memfs → virtio-blk → on-disk FAT or xv6-like → host-checkable image. Do not claim FAT. See [Filesystem: new vs extend](../overview/filesystem.md).

## Tracks (subordinate to principles)

[Track A](track-a.md) (freestanding apps, [#31](https://github.com/artofdream/ctos/issues/31)) and [Track B](track-b.md) (Linux-compat research, [#40](https://github.com/artofdream/ctos/issues/40)) do **not** override [principles.md](../framework/principles.md). A9 is after Track A ABI/loader.

## Docs website

mdBook + GitHub Pages (not a kernel milestone, not a new FR/NFR ID). `https://ctos.artof.link` HTTPS is **Verified** after #30 (deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200). Route 53 CNAME remains in place (zone `Z1178AFMV41RWP`). Overview: [what can run today](../overview/what-can-run.md). Publish notes: [website.md](../website.md).
