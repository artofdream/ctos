# Security — threat model v1 (NFR-10 / ADR-011 / ADR-012)

This is a **written threat model for a QEMU `virt` learning kernel**. It is not a certification, not an audit, and not a “secure OS” / “hardened” claim. File presence is not W^X. W^X is a separate ledger row that needs a QEMU probe.

Version: **v1** (2026-09-10). Supersedes the ADR-011 stub.

## Scope

**In:** the `ctos` guest as built for `qemu-system-aarch64 -machine virt` (EL1, identity map, PL011, GICv2, CNTP, first-fit heap, cooperative EL1 tasks). The git repo (no secrets).

**Out:** Raspberry Pi or other boards, a second ISA, networking, multi-tenant hosting, secure / measured boot, physical side channels, a hostile hypervisor.

QEMU and the host are the **TCB we do not defend against**. If the emulator or the CI runner is hostile, the guest cannot recover.

## Assets

| Asset | Why it matters | Where it lives |
| --- | --- | --- |
| Kernel integrity | Text, `VBAR_EL1` table, page tables. Corruption ends the learning probe. | EL1 identity map (`0x4008_0000`…`__kernel_end`) |
| Secrets-none | Credentials in git would be a host/CI incident, not a guest exploit. | Repo policy (NFR-10). The guest stores no keys. |
| Availability of the virt guest | Smoke / `cargo test` must still print markers and exit. A silent lockup hides bugs. | UART, GIC, timer, fail-closed `scripts/qemu-smoke.sh` |
| Heap + cooperative stacks | Writable RAM used by `Box`/`Vec` and M9 workers. Execute-from-here is the W^X question. | Frame pool after `__kernel_end` ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)) |
| Console / sensors | PL011 is how we see whether a probe ran. | Device MMIO `0x0900_0000` |

## Adversaries

| Adversary | In scope? | Notes |
| --- | --- | --- |
| **Buggy kernel code** (wrong store, bad `unsafe`, execute-from-heap) | **Yes** | Primary adversary today. Mitigate with PXN on heap/frames, minimize `unsafe` (NFR-01), fail-closed smoke. |
| **Malicious EL0** (future user task executing kernel data or escalating via a bad map) | **Named, not built** | Planned ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)). No EL0, no lower-EL map. Do not claim isolation. |
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
                                         EL0 user     ← Planned (not a boundary yet)
```

- **EL1 now.** Everything the kernel maps is one privilege. Page tables distinguish *execute* (kernel image vs heap/frames vs Device XN), not *who*.
- **EL0 Planned.** Lower-EL slots in `VBAR_EL1` still park. Closing probe (later): a lower-EL fetch of kernel data faults.
- **Repo vs guest.** “No secrets in repo” is a host boundary. It does not harden the UART.

## Non-goals

- Not a **certified** secure OS (no Common Criteria, no PSA, no “hardened”).
- Not **side-channel complete** (no cache/timing/Spectre story; QEMU TCG is the wrong lab).
- Not **W^X of the whole kernel image**. Linker SP_EL0 / SP_EL1 / fatal stacks and `.data`/`.bss` stay in executable pages with text. Heap + heap-backed cooperative stacks are the NX cut ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)).
- Not **stack guard pages** on the linker stacks (overflow there still smashes image, not a translation fault).
- Not secure boot, ASID isolation, or RO+NX text/data split (later ADRs).

## Mitigations mapped to probes

| Mitigation | Probe | Ledger |
| --- | --- | --- |
| Threat-model v1 written | Read this file | Verified (file + review). Still no “secure OS”. |
| Device MMIO XN (L1 block 0) | `pxn_for(0x0900_0000) == Some(true)` | Covered by the W^X tests when they run. |
| Heap + coop stacks PXN | Serial `wx: ok`; `#[test_case]` flags + execute-from-heap IABORT | Verified only after QEMU. Until then Unknown. |
| Kernel text still executable | `is_executable(0x4008_0000)` | Same W^X probe. |
| Execute-from-writable heap forbidden | Armed permission IABORT → `wx: nx heap` then `wx: ok` | Fail-closed in `scripts/qemu-smoke.sh`. |
| Minimize `unsafe` | Review (NFR-01). Count is not a proof. | Policy. |
| Fail-closed smoke | `scripts/qemu-smoke.sh` greps + `force-fail` exit ≠ 0 | NFR-04 / NFR-05. |
| No secrets in repo | Policy + `.gitignore` (including `.obsidian/`) | Host control. |
| IRQ least privilege | IRQ path does not allocate or `yield_now` | Convention. Ratchet if it fails twice. |
| EL0 isolation | Not built. Stub `src/el0.rs` + [el0.md](el0.md) | **Planned.** |

## Claim gate

A PR may say “threat-model v1 exists” after a file read. It may say “heap NX” / “W^X on the heap” only when the honesty ledger has a matching **Verified** QEMU probe (page-table PXN **and/or** a caught execute-from-heap abort).

It may **not** say “secure OS,” “hardened,” “EL0 works,” or “the kernel is W^X” (the linker stacks and kernel data are still executable). Unprobed stays **Unknown**.
