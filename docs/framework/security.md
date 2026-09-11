# Security — threat model v1.2 (NFR-10 / ADR-011 / ADR-012 / ADR-013 / ADR-014 / ADR-015)

This is a **written threat model for a QEMU `virt` learning kernel**. It is not a certification, not an audit, and not a “secure OS” / “hardened” claim. File presence is not W^X. Image W^X is a separate ledger row that needs a QEMU probe.

Version: **v1.2** (2026-09-11). Slice/update of v1.1 (adds RO+NX text/data, user TTBR0 read mile, remaining isolation gaps). Not a v2 model and not “secure.”

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
| Linker stacks (SP_EL0 / SP_EL1 / fatal) | Thread, first-level exception, and nested-fatal stacks. Live pages are RW+NX with `.data` ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). | Linker holes after `.bss` ([ADR-005](../03-adr/ADR-005-fatal-exception-stack.md)) |
| Guard pages (when mapped invalid) | 4 KiB unmapped holes below each linker stack. Downward overflow should become a translation fault, not a silent smash of the previous image bytes. | `__stack_guard` / `__exc_stack_guard` / `__fatal_stack_guard` ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md)) |
| EL0 trampoline + bait | One UXN-clear map-window page and a kernel `.data` bait. Proves enter/return, “cannot execute kernel data,” and (with user TTBR0) “cannot read kernel `.data`.” Not a user process, not isolation. | `paging::EL0_PAGE` + `KERNEL_DATA_BAIT` + `L1_USER` ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)) |
| Remaining isolation gaps | Shared kernel text in the user table (handler must fetch). No PAN on cortex-a57. ASID programmed but we TLBI ALL. No standing EL0. | User TTBR0 window; isolation Planned |
| Console / sensors | PL011 is how we see whether a probe ran. | Device MMIO `0x0900_0000` |

## Adversaries

| Adversary | In scope? | Notes |
| --- | --- | --- |
| **Buggy kernel code** (wrong store, bad `unsafe`, execute-from-heap, stack smash) | **Yes** | Primary adversary today. Mitigate with PXN on heap/frames, unmapped linker-stack guards, minimize `unsafe` (NFR-01), fail-closed smoke. |
| **Malicious EL0** (future user task executing kernel data or escalating via a bad map) | **Named, not built as a standing user** | First mile + user-TTBR0 read mile exist: `ERET` + SVC, IABORT on kernel `.data`, DABORT on an EL0 load of kernel `.data`. That is **not** isolation (kernel text still in the user table, no PAN, ASID not isolating TLBs, `is_active() == false`). |
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
                                         EL0 user         ← Planned (PAN / ASID isolation / standing task)
```

- **EL1 now.** Page tables distinguish *execute* (RO+X text vs RW+NX data/stacks/heap vs Device XN), *write* (AP[2] on text), and *presence* (guard holes).
- **EL0 first mile + user TTBR0, isolation Planned.** Lower-EL AArch64 **sync** is taken (SVC / IABORT / DABORT). IRQ/FIQ/SError lower-EL slots still park. Closing *isolation* still needs PAN and/or a higher-half / TTBR1 kernel, ASID-tagged TLB isolation, and a standing user. A caught load of `.data` is “cannot read kernel data,” not “isolated.”
- **Repo vs guest.** “No secrets in repo” is a host boundary. It does not harden the UART.

## Non-goals

- Not a **certified** secure OS (no Common Criteria, no PSA, no “hardened”).
- Not **side-channel complete** (no cache/timing/Spectre story; QEMU TCG is the wrong lab).
- Not a **product “the kernel is W^X”** sentence. The identity image on this virt guest is RO+X / RW+NX with WXN ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). Future mappings are not automatically covered. Guard pages remain **holes**.
- Not **ASAN / canaries / heap-stack guards**. Coop worker stacks have no unmapped holes. Overflow there is still image-adjacent PXN RAM.
- Not secure boot, PAN, ASID-tagged TLB isolation, or a higher-half kernel.

## Mitigations mapped to probes

| Mitigation | Probe | Ledger |
| --- | --- | --- |
| Threat-model v1.2 written | Read this file | Verified (file + review). Still no “secure OS”. |
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
| EL0 cannot execute kernel data | Serial `el0: nx kernel`; lower-EL IABORT | Verified first mile. |
| EL0 cannot read kernel `.data` | Serial `el0: no kernel read`; user TTBR0 omits `.data` | Verified read mile. Isolation stays **Planned**. |
| RO+NX text/data | Serial `ro: ok`; execute-from-`.data` + write-to-RO-text | See honesty ledger (this PR). |

## Claim gate

A PR may say “threat-model v1.2 exists” after a file read. It may say “heap NX” / “identity image is W^X on this virt guest” only when the honesty ledger has a matching **Verified** QEMU probe. It may say “linker-stack guard faults” only with a matching translation-abort probe. It may say “EL0 entered and returned,” “EL0 cannot execute kernel data,” or “EL0 cannot read kernel `.data`” only for the mile that actually passed.

It may **not** say “secure OS,” “hardened,” “EL0 works,” or “EL0 isolated.” Unprobed stays **Unknown**.
