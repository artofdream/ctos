# Security — threat model v1.8 (NFR-10 / ADR-011 / ADR-012 / ADR-013 / ADR-014 / ADR-015 / ADR-016 / ADR-017 / ADR-018 / ADR-019 / ADR-020)

This is a **written threat model for a QEMU `virt` learning kernel**. It is not a certification, not an audit, and not a “secure OS” / “hardened” claim. File presence is not W^X. Image W^X is a separate ledger row that needs a QEMU probe.

Version: **v1.8** (2026-09-11). Slice/update of v1.7 (adds high-VA vtable / fn-pointer rewrite + live identity `.text` tear after the boot stub; `.rodata`/`.data`/heap stay; PAN and full identity teardown still Planned). Not a v2 model and not “secure.”

## Scope

**In:** the `ctos` guest as built for `qemu-system-aarch64 -machine virt` (EL1, identity map + TTBR1 private page + TTBR1 RAM alias for EL1 fetch, PL011, GICv2, CNTP, first-fit heap, cooperative EL1 tasks, a deliberate EL0 first mile and a bounded standing context). The git repo (no secrets).

**Out:** Raspberry Pi or other boards, a second ISA, networking, multi-tenant hosting, secure / measured boot, physical side channels, a hostile hypervisor.

QEMU and the host are the **TCB we do not defend against**. If the emulator or the CI runner is hostile, the guest cannot recover.

## Assets

| Asset | Why it matters | Where it lives |
| --- | --- | --- |
| Kernel integrity | Text, `VBAR_EL1` table, page tables. Corruption ends the learning probe. | EL1 identity map (`0x4008_0000`…`__kernel_end`) plus TTBR1 RAM alias / high VBAR ([ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md)) |
| Secrets-none | Credentials in git would be a host/CI incident, not a guest exploit. | Repo policy (NFR-10). The guest stores no keys. |
| Availability of the virt guest | Smoke / `cargo test` must still print markers and exit. A silent lockup hides bugs. | UART, GIC, timer, fail-closed `scripts/qemu-smoke.sh` |
| Heap + cooperative stacks | Writable RAM used by `Box`/`Vec` and M9 workers. Execute-from-here is the W^X question. | Frame pool after `__kernel_end` ([ADR-012](../03-adr/ADR-012-wx-nx-heap-stacks.md)) |
| Linker stacks (SP_EL0 / SP_EL1 / fatal) | Thread, first-level exception, and nested-fatal stacks. Live pages are RW+NX with `.data` ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). | Linker holes after `.bss` ([ADR-005](../03-adr/ADR-005-fatal-exception-stack.md)) |
| Guard pages (when mapped invalid) | 4 KiB unmapped holes below each linker stack. Downward overflow should become a translation fault, not a silent smash of the previous image bytes. | `__stack_guard` / `__exc_stack_guard` / `__fatal_stack_guard` ([ADR-014](../03-adr/ADR-014-linker-stack-guard-pages.md)) |
| EL0 trampoline + bait + standing context | One UXN-clear map-window page and a kernel `.data` bait. Proves enter/return, “cannot execute kernel data,” “cannot read kernel `.data`,” and a bounded standing dual-SVC. Not POSIX, not isolation. | `paging::EL0_PAGE` + `KERNEL_DATA_BAIT` + `L1_USER` ([ADR-013](../03-adr/ADR-013-el0-isolation-direction.md)) |
| TTBR1 private page | One EL1-only high page. Proves EL0 cannot load it. Not a relocated kernel. | `paging::TTBR1_PRIV` ([ADR-016](../03-adr/ADR-016-ttbr1-private-page.md)) |
| TTBR1 RAM alias / high VBAR | EL1 can fetch `.text` at `identity + TTBR1_BASE`. `VBAR_EL1` is that alias. Identity `-kernel` stub stays. | `L1_HIGH[1]` → cloned `L2_HIGH_RAM` ([ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md), [ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md)) |
| Identity-tear page | One dedicated identity text page unmapped from TTBR0. High twin stays. Not a relocated kernel. | `__ident_tear_*` ([ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md)) |
| Identity text range | Dedicated 16 KiB identity text range unmapped. High twins stay. Not a relocated kernel. | `__ident_tear_*` 16 KiB ([ADR-019](../03-adr/ADR-019-identity-text-range-tear.md)) |
| Live identity `.text` | rustc vtables rewritten to high aliases; live identity `.text` after `_start` unmapped. High twins stay. `.rodata`/`.data`/heap stay. Not a relocated kernel. | `[0x4008_1000, __text_end)` ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)) |
| Remaining isolation gaps | Shared boot-stub text in the user table (handler must fetch if VBAR were still identity). No PAN on cortex-a57. EL0 trampoline still `TLBI VMALLE1` (`.data` leaves are global). Identity `.rodata`/`.data`/heap still live. Lower-EL IRQ still parks. | User TTBR0 + ASID + TTBR1 first cut + exec mile + torn live `.text`; isolation Planned |
| Console / sensors | PL011 is how we see whether a probe ran. | Device MMIO `0x0900_0000` |

## Adversaries

| Adversary | In scope? | Notes |
| --- | --- | --- |
| **Buggy kernel code** (wrong store, bad `unsafe`, execute-from-heap, stack smash) | **Yes** | Primary adversary today. Mitigate with PXN on heap/frames, unmapped linker-stack guards, minimize `unsafe` (NFR-01), fail-closed smoke. |
| **Malicious EL0** (standing or trampoline task executing kernel data or escalating via a bad map) | **Named; standing context is bounded** | First mile + user-TTBR0 read mile + ASID TLB mile + standing dual-SVC + TTBR1 private page + EL1 high-VA fetch exist. That is **not** “EL0 isolated” (kernel text still in the user table, no PAN, identity map still live, EL0 path still full-TLBI, lower-EL IRQ still parks). |
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
                                         EL0 first mile + standing  ← entered / stood / left
                                              |                       (not a POSIX user)
                                         EL0 isolation    ← Planned (PAN / full higher-half)
```

- **EL1 now.** Page tables distinguish *execute* (RO+X text vs RW+NX data/stacks/heap vs Device XN), *write* (AP[2] on text), and *presence* (guard holes).
- **EL0 first mile + user TTBR0 + ASID TLB + standing + TTBR1 first cut + EL1 high-VA fetch + identity-tear first cut + identity `.text` range tear + live `.text` tear, isolation Planned.** Lower-EL AArch64 **sync** is taken (SVC / IABORT / DABORT). IRQ/FIQ/SError lower-EL slots still park. Dual ASID without `TLBI VMALLE1` is a probed mile (`asid: ok`). Standing dual-SVC flips `is_active()`. TTBR1 private page is EL1-only (`ttbr1: ok`). EL1 can fetch a real path from the high RAM alias (`ttbr1: el1 exec`); `VBAR_EL1` is that alias (`ttbr1: vbar`). A 16 KiB dedicated identity text range is unmapped (`ident: range`); live identity `.text` after `_start` is unmapped after a vtable rewrite (`ident: reloc` / `ident: live`); `_start` stays. Closing *isolation* still needs PAN and a full identity teardown. A caught load of `.data` or of `TTBR1_PRIV` is that mile, not “EL0 isolated.”
- **Repo vs guest.** “No secrets in repo” is a host boundary. It does not harden the UART.

## Non-goals

- Not a **certified** secure OS (no Common Criteria, no PSA, no “hardened”).
- Not **side-channel complete** (no cache/timing/Spectre story; QEMU TCG is the wrong lab).
- Not a **product “the kernel is W^X”** sentence. The identity image on this virt guest is RO+X / RW+NX with WXN ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). Future mappings are not automatically covered. Guard pages remain **holes**.
- Not **ASAN / canaries / heap-stack guards**. Coop worker stacks have no unmapped holes. Overflow there is still image-adjacent PXN RAM.
- Not secure boot, PAN, or a fully torn-down identity map (ADR-020 unmaps live identity `.text` after a high-VA vtable rewrite; `.rodata`/`.data`/heap stay; `_start` stays at `0x4008_0000`).

## Mitigations mapped to probes

| Mitigation | Probe | Ledger |
| --- | --- | --- |
| Threat-model v1.8 written | Read this file | Verified (file + review). Still no “secure OS”. |
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
| ASID isolation (no `VMALLE1`) | Serial `asid: dual` / `asid: conflict` / `asid: ok` | Specific mile. Umbrella isolation stays **Planned**. |
| Standing EL0 context | Serial `el0: standing` / `el0: restored`; `is_active()` flips | Specific mile. Not POSIX. Not isolation. |
| TTBR1 private page | Serial `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok` | First cut ([ADR-016](../03-adr/ADR-016-ttbr1-private-page.md)). |
| EL1 fetch from TTBR1 high VA | Serial `ttbr1: el1 exec` / `ttbr1: vbar` | Exec mile ([ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md)). |
| Identity-tear first cut | Serial `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok` | First cut ([ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md)). Full teardown Planned. |
| Identity text range tear | Serial `ident: jump` / `ident: range` / `ident: text` | Range cut ([ADR-019](../03-adr/ADR-019-identity-text-range-tear.md)). |
| High-VA vtable rewrite + live `.text` tear | Serial `ident: reloc` / `ident: live` | Live `.text` cut ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)). `.rodata`/`.data`/heap stay. |
| RO+NX text/data | Serial `ro: ok`; execute-from-`.data` + write-to-RO-text | Verified: 2026-09-11 cloud `qemu-smoke` (honesty ledger). |

## Claim gate

A PR may say “threat-model v1.8 exists” after a file read. It may say “heap NX” / “identity image is W^X on this virt guest” only when the honesty ledger has a matching **Verified** QEMU probe. It may say “linker-stack guard faults” only with a matching translation-abort probe. It may say “EL0 entered and returned,” “EL0 cannot execute kernel data,” “EL0 cannot read kernel `.data`,” “standing EL0 context,” “ASID isolation,” “TTBR1 private page,” “EL1 fetched from a TTBR1 high VA,” “one identity text page was unmapped,” “a 16 KiB dedicated identity text range was unmapped,” “rustc vtables were rewritten to high aliases,” or “live identity `.text` after the boot stub was unmapped” only for the mile that actually passed.

It may **not** say “secure OS,” “hardened,” “EL0 works,” “EL0 isolated,” “the kernel moved,” or “immutable OS.” Scoped RO is [immutability.md](immutability.md). Unprobed stays **Unknown**. https://ctos.artof.link HTTPS is **Verified** (2026-09-11 after #30; see the [honesty ledger](honesty-ledger.md)).
