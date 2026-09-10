# Security — threat model v1.1 (NFR-10 / ADR-011 / ADR-012 / ADR-013 / ADR-014)

This is a **written threat model for a QEMU `virt` learning kernel**. It is not a certification, not an audit, and not a “secure OS” / “hardened” claim. File presence is not W^X. W^X is a separate ledger row that needs a QEMU probe.

Version: **v1.1** (2026-09-10). Slice/update of v1 (same non-goals except guard pages, which [ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md) now covers). Not a v2 model and not “secure.”

## Scope

**In:** the `ctos` guest as built for `qemu-system-aarch64 -machine virt` (EL1, identity map, PL011, GICv2, CNTP, first-fit heap, cooperative EL1 tasks, a deliberate EL0 first mile). The git repo (no secrets).

**Out:** Raspberry Pi or other boards, a second ISA, networking, multi-tenant hosting, secure / measured boot, physical side channels, a hostile hypervisor.

QEMU and the host are the **TCB we do not defend against**. If the emulator or the CI runner is hostile, the guest cannot recover.

## Assets

| Asset | Why it matters | Where it lives |
| --- | --- | --- |
| Kernel integrity | Text, `VBAR_EL1` table, page tables. Corruption ends the learning probe. | EL1 identity map (`0x4008_0000`…`__kernel_end`) |
| Secrets-none | Credentials in git would be a host/CI incident, not a guest exploit. | Repo policy (NFR-10). The guest stores no keys. |
| Availability of the virt guest | Smoke / `cargo test` must still print markers and exit. A silent lockup hides bugs. | UART, GIC, timer, fail-closed `scripts/qemu-smoke.sh` |
| Heap + cooperative stacks | Writable RAM used by `Box`/`Vec` and M9 workers. Execute-from-here is the W^X question. | Frame pool after `__kernel_end` ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)) |
| Linker stacks (SP_EL0 / SP_EL1 / fatal) | Thread, first-level exception, and nested-fatal stacks. They still live in the **executable** image with text / `.data` / `.bss`. | Linker holes after `.bss` ([ADR-005](../03-adr/ADR-005-fatal-exception-stack.md)) |
| Guard pages (when mapped invalid) | 4 KiB unmapped holes below each linker stack. Downward overflow should become a translation fault, not a silent smash of the previous image bytes. | `__stack_guard` / `__exc_stack_guard` / `__fatal_stack_guard` ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md)) |
| EL0 first mile (trampoline + bait) | One UXN-clear map-window page and a kernel `.data` bait. Proves enter/return and “cannot execute kernel data.” Not a user process, not an isolated map. | `paging::EL0_PAGE` + `KERNEL_DATA_BAIT` ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)) |
| Remaining W^X gaps | Image stacks, `.data`, `.bss`, and page tables stay executable with text. `SCTLR.WXN` is still off. Heap + coop stacks are the NX cut; guard holes are unmapped, not NX stacks. | Kernel-image pages (`[0x4008_0000, align_4k(__kernel_end))` minus the three guard VAs) |
| Console / sensors | PL011 is how we see whether a probe ran. | Device MMIO `0x0900_0000` |

## Adversaries

| Adversary | In scope? | Notes |
| --- | --- | --- |
| **Buggy kernel code** (wrong store, bad `unsafe`, execute-from-heap, stack smash) | **Yes** | Primary adversary today. Mitigate with PXN on heap/frames, unmapped linker-stack guards, minimize `unsafe` (NFR-01), fail-closed smoke. |
| **Malicious EL0** (future user task executing kernel data or escalating via a bad map) | **Named, not built as a standing user** | A first mile exists: deliberate `ERET` + SVC return, and a lower-EL UXN IABORT on kernel `.data`. That is **not** isolation (shared `TTBR0`, no PAN, no ASID, `is_active() == false`). Do not treat this adversary as present until those remaining probes exist. |
| **Compromised device tree** | **Mostly out** | M7 does not walk FDT. DTB sits at RAM base below the image. A hostile DTB is a QEMU/host problem until a walker exists; then it becomes an input-validation ADR. |
| **DMA / virtio devices** | **Out** | The guest does not program a DMA master. UART/GIC/timer are MMIO. A malicious virtio device is future surface. |
| **Hostile QEMU or CI host** | **Out** | Hypervisor / runner is trusted. Repo-secret leak is a *host* control (`.gitignore`, review), not a guest mitigation. |
| **Network attacker** | **Out** | No stack. |

## Trust boundaries

```
[ host / QEMU / CI ]  --trusted config--  [ virt guest ]
                                              |
                                         EL1 kernel  ← TCB today
                                              |
                                         EL0 first mile  ← entered and left on purpose
                                              |            (not a standing boundary)
                                         EL0 user         ← Planned (separate map / PAN / ASID)
```

- **EL1 now.** Everything the kernel maps is one privilege. Page tables distinguish *execute* (kernel image vs heap/frames vs Device XN) and *presence* (guard holes), not *who*.
- **EL0 first mile, isolation Planned.** Lower-EL AArch64 **sync** is taken (SVC / IABORT). IRQ/FIQ/SError lower-EL slots still park. Closing *isolation* still needs a user map ≠ kernel identity window (and usually PAN / ASID). A caught UXN fetch is “cannot execute kernel data,” not “isolated.”
- **Repo vs guest.** “No secrets in repo” is a host boundary. It does not harden the UART.

## Non-goals

- Not a **certified** secure OS (no Common Criteria, no PSA, no “hardened”).
- Not **side-channel complete** (no cache/timing/Spectre story; QEMU TCG is the wrong lab).
- Not **W^X of the whole kernel image**. Linker SP_EL0 / SP_EL1 / fatal **pages** and `.data`/`.bss` stay in executable pages with text. Heap + heap-backed cooperative stacks are the NX cut ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)). Guard pages are **holes**, not NX stacks.
- Not **ASAN / canaries / heap-stack guards**. Coop worker stacks have no unmapped holes. Overflow there is still image-adjacent PXN RAM.
- Not secure boot, ASID isolation, PAN, or RO+NX text/data split (later ADRs).

## Mitigations mapped to probes

| Mitigation | Probe | Ledger |
| --- | --- | --- |
| Threat-model v1.1 written | Read this file | Verified (file + review). Still no “secure OS”. |
| Device MMIO XN (L1 block 0) | `pxn_for(0x0900_0000) == Some(true)` | Covered by the W^X tests when they run. |
| Heap + coop stacks PXN | Serial `wx: ok`; `#[test_case]` flags + execute-from-heap IABORT | Verified: 2026-09-10 cloud `qemu-smoke` (honesty ledger). |
| Kernel text still executable | `is_executable(0x4008_0000)` | Same W^X probe. |
| Execute-from-writable heap forbidden | Armed permission IABORT → `wx: nx heap` then `wx: ok` | Fail-closed in `scripts/qemu-smoke.sh`. |
| Linker-stack guard holes | Serial `guard: fault` / `guard: ok`; store to `__stack_guard` | Verified: 2026-09-10 cloud `qemu-smoke` (honesty ledger). |
| Minimize `unsafe` | Review (NFR-01). Count is not a proof. | Policy. |
| Fail-closed smoke | `scripts/qemu-smoke.sh` greps + `force-fail` exit ≠ 0 | NFR-04 / NFR-05. |
| No secrets in repo | Policy + `.gitignore` (including `.obsidian/`) | Host control. |
| IRQ least privilege | IRQ path does not allocate or `yield_now` | Convention. Ratchet if it fails twice. |
| EL0 entered and returned | Serial `el0: svc` / `el0: ok`; `#[test_case]` | Verified first mile — not isolation. |
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; lower-EL UXN IABORT | Verified first mile. Isolation (separate map) stays **Planned**. |

## Claim gate

A PR may say “threat-model v1.1 exists” after a file read. It may say “heap NX” / “W^X on the heap” only when the honesty ledger has a matching **Verified** QEMU probe (page-table PXN **and/or** a caught execute-from-heap abort). It may say “linker-stack guard faults” only with a matching translation-abort probe. It may say “EL0 entered and returned” or “EL0 cannot execute kernel data” only for the mile that actually passed.

It may **not** say “secure OS,” “hardened,” “EL0 works,” “EL0 isolated,” or “the kernel is W^X” (the linker stack **pages** and kernel data are still executable). Unprobed stays **Unknown**.
