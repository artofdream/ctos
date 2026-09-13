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

M0–M9 are on `main` (M9 = merge of PR #15 / FR-11). Pillar work through ADR-020 (#17–#28) plus docs #30/#51/#29, A1–A6 (ADR-021–ADR-027) are on `main`. This PR is Track A / A7 ([issue #38](https://github.com/artofdream/ctos/issues/38), parent [issue #31](https://github.com/artofdream/ctos/issues/31)): ADR-028 virtio-blk + FAT16. Merge is still a human/MRC job (ADR-002).

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
| P-SEC-3d | Standing EL0 context | `el0: standing` + `el0: restored`; `is_active()` true only while standing | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Not POSIX. Default `ERET` clears IRQ mask (ADR-041). |
| P-SEC-3e | TTBR1 kernel-private page (ADR-016 first cut) | `ttbr1: el1` + `ttbr1: no el0` + `ttbr1: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). |
| P-SEC-3f | EL1 fetch from TTBR1 high VA (ADR-017) | `ttbr1: el1 exec` + `ttbr1: vbar` + existing `ttbr1: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Identity boot stub stays. |
| P-SEC-3g | Identity-tear first cut (ADR-018) | `ident: split` + `ident: fault` + `ident: high` + `ident: no el0` + `ident: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` (see honesty ledger). Full identity teardown / PAN / umbrella isolation stay **Planned**. Do not claim “the kernel moved.” |
| P-SEC-3h | High-VA jump + 16 KiB identity text range (ADR-019) | `ident: jump` + `ident: range` + `ident: text` + existing `ident: ok` | **Verified:** 2026-09-11 cloud `qemu-smoke` + cts-ai Docker on `24d94e6` (see honesty ledger). Do not claim “the kernel moved.” |
| P-SEC-3i | High-VA vtable rewrite + live identity `.text` tear (ADR-020) | `ident: reloc` + `ident: live` + existing `ident: ok`; `println!` after the tear | **Verified:** 2026-09-11 cloud `qemu-smoke` + cts-ai Docker on `e80dc93` + GHA merge-commit [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) (see honesty ledger). `.rodata` / `.data` / heap stay. Do not claim “the kernel moved.” |
| P-SEC-3j | Identity `.rodata` / `.data` / heap tear | `.rodata` + `.data`/stacks + heap unmapped + high-only access. | **`.rodata` Verified** (ADR-025). **`.data`/stacks Verified** (ADR-037). **Heap Verified** (ADR-038) on this branch `qemu-smoke` (`ident: heap-reloc` / `ident: heap` / `ident: heap-fault` / `ident: heap-high`; heap-stay gone). Do not claim “the kernel moved.” |
| P-SEC-3k | PAN on virt `cortex-a57` | `ID_AA64MMFR1_EL1.PAN != 0` **and** an EL1-vs-EL0 access fault | **Planned** (ADR-026). ID-field probe prints `pan: absent` on `-cpu cortex-a57`. Do not switch `-cpu` silently. |
| P-SEC-3m | EL0 entry without `TLBI VMALLE1` (ADR-039) | `el0: no-vmalle1`; `src/exception.rs` has no `tlbi vmalle1` | **Verified:** 2026-09-12 agent-box `qemu-smoke` (see honesty ledger). Not “EL0 isolated.” PAN / umbrella stay **Planned**. |
| P-SEC-3n | Lower-EL IRQ while standing (ADR-040) | `el0: irq`; `#[test_case]` `lower_el_irq_while_standing` | **Verified** when this tip’s `qemu-smoke` prints `el0: irq` (see honesty ledger). FIQ/SError still park. Not “EL0 isolated.” |
| P-SEC-3o | IRQ-unmasked default standing/task `ERET` (ADR-041) | `el0: irq-default`; `#[test_case]` `lower_el_irq_default_eret` | **Verified** when this tip’s `qemu-smoke` prints `el0: irq-default` (see honesty ledger). Short non-standing probes stay masked. FIQ/SError still park. Not “EL0 isolated.” |
| P-SEC-3p | Identity `_start` / leftover-RAM stay honesty (ADR-042) | `ident: start-stay` + `ident: ram-stay`; `#[test_case]` `identity_boot_stub_stays_while_live_torn` | **Verified** when this tip’s `qemu-smoke` prints `ident: start-stay` (see honesty ledger). Honesty wrap — not a teardown. Do not yank `_start`. Not “EL0 isolated.” |
| P-SEC-3q | Lower-EL FIQ while standing + SError park (ADR-043) | `el0: fiq` + `el0: serror-park`; `#[test_case]` `lower_el_fiq_while_standing` | **Verified** (FIQ taken) / **Planned** (taken SError) when this tip’s `qemu-smoke` prints both markers (see honesty ledger). Not “EL0 isolated.” |
| P-SEC-3l | Umbrella EL0 isolation | Standing + PAN + full TTBR1 / identity teardown | **Planned** ([ADR-042](../03-adr/ADR-042-isolation-leftover-wrap.md)). Specific miles (P-SEC-3…P-SEC-3q) are not this row. Do not claim “EL0 isolated.” |

Hub: [pillars.md](../framework/pillars.md). ABI contract: [syscall.md](../framework/syscall.md).

## Track A — freestanding app hosting ([#31](https://github.com/artofdream/ctos/issues/31))

A1 is the SVC ABI mile. A2 is the CRT / `libctos` mile. A3 is the guest ELF PT_LOAD loader mile. A4 is standing EL0 as **normal** mode for a loaded image. A5 is isolation completion (identity `.rodata` / `.data` / heap tears + EL0 entry without `VMALLE1` + PAN ID-field; PAN enable Planned). A6 is thin VFS + in-RAM memfs. A7 is virtio-blk + FAT16. A8 is documented sample **rebuild recipes** (no new runtime). A9 is the OS/app **slot first cut** plus leftover cross-update / embed-off ([ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). **Track A children A1–A9 have a first cut**; “app hosting is done” stays unclaimed. Not Linux containers. Not glibc. One child issue → one PR. Do not round A1–A9 Verified up to “app hosting is done.”

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| A1 | Stable SVC ABI + docs ([#32](https://github.com/artofdream/ctos/issues/32), ADR-021) | `svc: yield` + `svc: user-hi` + `svc: uart` + `svc: exit` + `svc: ok`; `#[test_case]` | **ABI mile Verified:** 2026-09-11 cloud `qemu-smoke` on `8846bc5` (onto `aa46219` / #29). App hosting stays **Planned**. |
| A2 | Freestanding CRT / `libctos` ([#33](https://github.com/artofdream/ctos/issues/33), ADR-022) | `libctos: hi` + `libctos: ok` + `libctos: linked`; `#[test_case]` | **CRT mile Verified:** 2026-09-12 cloud `qemu-smoke` on `c9b292b`. App hosting stays **Planned**. |
| A3 | ELF (or raw image) loader into user TTBR0 ([#34](https://github.com/artofdream/ctos/issues/34), ADR-023) | `loader: mapped` + `loader: ok`; payload `libctos: hi` / `libctos: ok`; `#[test_case]` | **Loader mile Verified:** 2026-09-12 cloud `qemu-smoke` (see honesty ledger). Not a Linux ABI. App hosting stays **Planned**. |
| A4 | Standing EL0 as normal mode ([#35](https://github.com/artofdream/ctos/issues/35), ADR-024) | `el0: task-enter` + `el0: task-active` + `el0: task-exit` + `el0: task-restored` + `el0: restore-fail` + `el0: task-ok`; `#[test_case]` | **Standing-task mile Verified:** 2026-09-12 cloud `qemu-smoke` (see honesty ledger). Not isolation. App hosting stays **Planned**. |
| A5 | Isolation completion ([#36](https://github.com/artofdream/ctos/issues/36), ADR-025 / ADR-026 / ADR-032 / ADR-037 / ADR-038 / ADR-039 / ADR-040 / ADR-041 / ADR-042 / ADR-043) | `ident: rodata` + `ident: data` + `ident: heap`; `el0: no-vmalle1`; `el0: irq` + `el0: irq-default` + `el0: fiq` + `el0: serror-park`; `pan: id=` + `pan: absent` | **`.rodata` + `.data` + heap + EL0 no-VMALLE1 + lower-EL IRQ + default IRQ-unmasked ERET + lower-EL FIQ + SError park honesty + PAN ID-field Verified** on this branch `qemu-smoke`. Taken SError + PAN enable **Planned**. Not “EL0 isolated.” |
| A6 | Thin VFS + memfs ([#37](https://github.com/artofdream/ctos/issues/37), ADR-027) | `fs: create` + `fs: write` + `fs: read` + `fs: el0` + `fs: ok`; `#[test_case]` | **memfs mile Verified** (see honesty ledger). Not POSIX. App hosting stays **Planned**. |
| A7 | virtio-blk + FAT16 ([#38](https://github.com/artofdream/ctos/issues/38), ADR-028) | `blk: virtio` + `blk: cap` + `blk: rw` + `blk: ok`; `fat: mount` + `fat: read` + `fat: ok`; `#[test_case]` | **block + FAT mile Verified** when this tip’s `qemu-smoke` passes (see honesty ledger). Same VFS `open`. Not POSIX. Not FAT32. App hosting stays **Planned**. |
| A8 | Documented sample apps ([#39](https://github.com/artofdream/ctos/issues/39)) | recipes cite only existing serial markers; docs-build | **Recipes Verified** (docs; smoke markers unchanged). Hub: [what-can-run.md](../overview/what-can-run.md). No new ADR. Not app hosting. |
| A9 | OS–app slots ([#48](https://github.com/artofdream/ctos/issues/48), ADR-030 / ADR-032) | host OS ELF + `hello-libctos.elf`; FAT `/hello`; `slot: ok`; same ELF on this OS and `ba6541c` | **First cut + leftover** when this tip’s `qemu-smoke` prints `cross-update` lines (see honesty ledger). A2–A4 load FAT (no embed). Not app hosting. |

On-disk filesystem work is this A7 mile (FAT16, not xv6-like). See [Filesystem: new vs extend](../overview/filesystem.md).

## Tracks (subordinate to principles)

[Track A](track-a.md) (freestanding apps, [#31](https://github.com/artofdream/ctos/issues/31)) and [Track B](track-b.md) (Linux-compat research, [#40](https://github.com/artofdream/ctos/issues/40)) do **not** override [principles.md](../framework/principles.md). A9 is after Track A ABI/loader. Track B B1: Linux-compat **subset** research frame ([ADR-031](../03-adr/ADR-031-linux-compat-goals.md)) — **not claiming Linux userspace yet**. Track B B2: Linux AArch64 vs ctos SVC gap map ([linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md)) — inspection only; no Linux numbers in `src/`. Track B B3: standing EL0 stays the ctos process model ([ADR-035](../03-adr/ADR-035-process-model-standing-el0.md)) — Linux `clone`/`execve`/`wait4` is a gap; do not add those syscalls. Track B B4: Linux ELF / auxv / `PT_INTERP` vs the freestanding loader ([ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md)) — gap ADR; Track A loader stays reusable; **not claiming dynamic Linux ELF**. Track B B5: Linux VFS vs thin ctos VFS ([ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md)) — concept compare only; no POSIX flags / dentries. Track B B6: **never** Linux-compat implementation from this ladder ([ADR-036](../03-adr/ADR-036-linux-compat-decision.md)). Track B B7: guest OCI/Docker is a **non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)).

## Docs website

mdBook + GitHub Pages (not a kernel milestone, not a new FR/NFR ID). `https://ctos.artof.link` HTTPS is **Verified** after #30 (deploy [34653046584](https://github.com/artofdream/ctos/actions/runs/34653046584) + HTTPS 200). Route 53 CNAME remains in place (zone `Z1178AFMV41RWP`). Overview: [what can run today](../overview/what-can-run.md). Publish notes: [website.md](../website.md).

### Isolation leftover (post ADR-043)

- Taken SError: research [ADR-044](../03-adr/ADR-044-taken-serror-research.md) — implement QMP/`nmi` probe as a later ADR; still **Planned**.
- PAN enable on `-cpu cortex-a57`: still **Planned** (absent; do not switch `-cpu`).
- Yank `_start` / leftover identity RAM: still **Planned** (boot stub stays).
- Umbrella “EL0 isolated”: still **Planned**.