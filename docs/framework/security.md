# Security — threat model v1.49 (NFR-10 / ADR-011 / ADR-012 / ADR-013 / ADR-014 / ADR-015 / ADR-016 / ADR-017 / ADR-018 / ADR-019 / ADR-020 / ADR-021 / ADR-022 / ADR-023 / ADR-024 / ADR-025 / ADR-026 / ADR-027 / ADR-028 / ADR-030 / ADR-032 / ADR-037 / ADR-038 / ADR-039 / ADR-040 / ADR-041 / ADR-042 / ADR-043 / ADR-044 / ADR-045 / ADR-047 / ADR-048 / ADR-049 / ADR-050 / ADR-052 / ADR-053 / ADR-054 / ADR-055 / ADR-056 / ADR-057 / ADR-058 / ADR-059 / ADR-060 / ADR-061 / ADR-062 / ADR-063 / ADR-064 / ADR-065 / ADR-066 / ADR-067 / ADR-068 / ADR-069 / ADR-070 / ADR-071 / ADR-072 / ADR-073 / ADR-074)

This is a **written threat model for a QEMU `virt` learning kernel**. It is not a certification, not an audit, and not a “secure OS” / “hardened” claim. File presence is not W^X. Image W^X is a separate ledger row that needs a QEMU probe.

Version: **v1.49** (2026-09-16). Slice/update of v1.48 (#119 ADR-073). Thin TCP N5 ([ADR-074](../03-adr/ADR-074-n5-thin-tcp.md): `net: tcp-ok` via QEMU `guestfwd` echo). Keep N1/N2/N3/N3.x/N4 + ADR-069/072/073. Still **not** BSD sockets / TCP product / listen-accept / TLS/HTTP / DHCP/DNS product / Wi‑Fi / “has networking / sockets OS.” Still not Linux/POSIX/`mount`/`unlink`/`mkdir`/`getdents`/containers/preemption/exFAT. Not a v2 model and not “secure.”

## Scope

**In:** the `ctos` guest as built for `qemu-system-aarch64 -machine virt` (EL1, identity map + TTBR1 private page + TTBR1 RAM alias for EL1 fetch, PL011, GICv2, CNTP, first-fit heap, cooperative EL1 tasks, a deliberate EL0 first mile and a bounded standing context). The git repo (no secrets).

**Out:** Raspberry Pi or other boards, a second ISA, multi-tenant hosting, secure / measured boot, physical side channels, a hostile hypervisor. **Networking:** N1 ARP + N2 ICMP ping + N3 minimal UDP + N3.x EL0 UDP DNS SVC + N4 EL0 net SVCs + N5 thin TCP ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) / [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) / [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) / [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md) / [ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md) / [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md)); not a BSD sockets / TCP product stack.

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
| Live identity `.text` | rustc vtables rewritten to high aliases; live identity `.text` after `_start` unmapped. High twins stay. Not a relocated kernel. | `[0x4008_1000, __text_end)` ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)) |
| Identity `.rodata` | Identity `.rodata` unmapped after a pointer rewrite. High twin stays. Not a relocated kernel. | `[__rodata_start, __ident_tear_start)` ([ADR-025](../03-adr/ADR-025-identity-rodata-tear.md)) |
| Identity `.data` / stacks | Identity `.data`/`.bss`/linker stacks unmapped after SP relocate + pointer rewrite. High twin stays. Not a relocated kernel. | `[__data_start, __kernel_end)` ([ADR-037](../03-adr/ADR-037-identity-data-tear.md)) |
| Identity heap | Identity heap unmapped after high `GlobalAlloc` VAs. High twin stays. Remaining frames after the heap stay identity-mapped. Not a relocated kernel. | 64 KiB first-fit pool ([ADR-038](../03-adr/ADR-038-identity-heap-tear.md)) |
| PAN capability | `ID_AA64MMFR1_EL1.PAN` printed. Enable is **non-goal** on default cortex-a57 ([ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). | `src/pan.rs` ([ADR-026](../03-adr/ADR-026-pan-capability.md)) |
| Remaining isolation gaps | Shared boot-stub text stays by decision (`ident: start-stay`; never yank `_start`). PAN enable **locked** non-goal on default cortex-a57 ([ADR-054](../03-adr/ADR-054-pan-enable-lock.md)). Leftover identity RAM torn (`ident: ram*`, [ADR-049](../03-adr/ADR-049-identity-ram-tear.md)). Taken lower-EL SError **hard-stopped** (`el0: serror-park`; A-clear dormant prep; [ADR-053](../03-adr/ADR-053-taken-serror-hard-stop.md)). | User TTBR0 + ASID + TTBR1 + torn live `.text` / `.rodata` / `.data`/stacks / heap / leftover frame RAM + EL0 entry without `VMALLE1` + lower-EL IRQ while standing + default IRQ-unmasked ERET + lower-EL FIQ while standing + SError park honesty + A-clear dormant prep + start-stay honesty + SVC ABI; umbrella isolation Planned/non-claim until checklist ([ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md) / [ADR-055](../03-adr/ADR-055-el0-isolated-checklist.md)); sponsor closure table [ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md) |
| SVC ABI (exit / uart_write / yield) | Documented numbers 16–18. `uart_write` rejects a kernel `.data` pointer and a TTBR1 / non-canonical alias. Not Linux. Not app hosting. | `src/syscall.rs` ([ADR-021](../03-adr/ADR-021-svc-syscall-abi.md)) |
| `libctos` CRT | Wrappers + `_start` CRT. Hello payload copied onto the standing EL0 page. Must not issue SVC #0–#2. | `libctos/` + `src/libctos.rs` ([ADR-022](../03-adr/ADR-022-libctos-crt.md)) |
| Guest ELF PT_LOAD loader | Guest parses embedded ELF64 `ET_EXEC`, maps `PT_LOAD` into the user map-window, `ERET`s to `e_entry`. Rejects `PT_INTERP` and W+X. Not a Linux ABI. | `src/loader.rs` ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)) |
| Standing EL0 as normal mode | Loaded image stands until `SYS_EXIT`. `is_active()` is a task flag. Unexpected lower-EL sync restores fail-closed. Not a process table. | `src/el0.rs` ([ADR-024](../03-adr/ADR-024-standing-el0-normal.md)) |
| Thin VFS + memfs | Named heap buffers. Create / open / read / write / close. User path/I/O pointers must be user-mapped **and** kernel-mapped. Not POSIX. | `src/vfs.rs` ([ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)) |
| virtio-net (N1 ARP + N2 ICMP + N3 UDP + N5 thin TCP + N3.x/N4 EL0 SVC) | Guest programs a virtio-mmio net DMA master for ARP, ICMP echo, one UDP datagram, then one thin TCP active-open + payload vs QEMU SLIRP/`guestfwd`. EL0 uses tiny SVCs only (kernel owns NIC). Not a BSD sockets / TCP product. | `src/virtio.rs` + `src/syscall.rs` ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md), [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md), [ADR-070](../03-adr/ADR-070-n3-udp-transport.md), [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md), [ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md), [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md)) |
| virtio-blk + FAT16 | Guest programs a virtio-mmio DMA master. Image is host-built FAT16. Guest may write/create small root files ([ADR-050](../03-adr/ADR-050-fat16-write.md)), grow a file across a cluster boundary ([ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md)), list root names ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)), delete a root file ([ADR-057](../03-adr/ADR-057-fat16-delete.md)), and create a root directory ([ADR-073](../03-adr/ADR-073-fat16-mkdir.md)). Same VFS `open`/`write`/`readdir`/`unlink`/`mkdir` behind prefix mounts ([ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md)). Not a trusted disk. | `src/virtio.rs` + `src/fat.rs` + `src/vfs.rs` ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md), [ADR-050](../03-adr/ADR-050-fat16-write.md), [ADR-056](../03-adr/ADR-056-fat16-readdir.md), [ADR-057](../03-adr/ADR-057-fat16-delete.md), [ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md), [ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md), [ADR-073](../03-adr/ADR-073-fat16-mkdir.md)) |
| OS/app slots | Host kernel ELF + published app ELF. Guest reads FAT `/hello` and maps `PT_LOAD`. A2–A4 use the same file (no embed). Cross-update is a host two-boot probe. Not a trusted disk. | `src/slot.rs` ([ADR-030](../03-adr/ADR-030-os-app-slots.md), [ADR-032](../03-adr/ADR-032-track-a-leftovers.md)) |
| Console / sensors | PL011 is how we see whether a probe ran. | Device MMIO `0x0900_0000` |

## Adversaries

| Adversary | In scope? | Notes |
| --- | --- | --- |
| **Buggy kernel code** (wrong store, bad `unsafe`, execute-from-heap, stack smash) | **Yes** | Primary adversary today. Mitigate with PXN on heap/frames, unmapped linker-stack guards, minimize `unsafe` (NFR-01), fail-closed smoke. |
| **Malicious EL0** (standing or trampoline task executing kernel data or escalating via a bad map) | **Named; standing context is bounded** | First mile + user-TTBR0 read mile + ASID TLB mile + standing dual-SVC + standing **task** until `exit` + TTBR1 private page + EL1 high-VA fetch + a documented SVC ABI exist. That is **not** “EL0 isolated” (boot-stub identity page stays by decision — `ident: start-stay`; PAN enable non-goal on default a57; leftover identity RAM torn — `ident: ram*`; taken SError deferred/non-goal — `el0: serror-park` / [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). Product freestanding hosting Verified under ADR-048/052; not Linux/POSIX/containers ([ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) / [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md)). |
| **Compromised device tree** | **Mostly out** | M7 does not walk FDT. DTB sits at RAM base below the image. A hostile DTB is a QEMU/host problem until a walker exists; then it becomes an input-validation ADR. |
| **DMA / virtio devices** | **Named; bounded** | A7 programs virtio-mmio blk (one queue, poll `used.idx`, identity-PA DMA). N1–N3/N5 program virtio-mmio net (RX+TX queues, same poll/DMA pattern) for ARP, ICMP echo, one UDP datagram, then thin TCP ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) / [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) / [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) / [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md)). QEMU is still TCB. A malicious virtio device is out (hostile hypervisor). No BSD sockets / TCP product. |
| **Hostile QEMU or CI host** | **Out** | Hypervisor / runner is trusted. Repo-secret leak is a *host* control (`.gitignore`, review), not a guest mitigation. |
| **Network attacker** | **Named; bounded (link only)** | N1–N3/N5 attach QEMU user netdev (+ N5 `guestfwd` echo) + guest virtio-net for ARP + ICMP + one UDP datagram + thin TCP ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) / [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) / [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) / [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md)). No BSD sockets / TCP product. QEMU SLIRP/`guestfwd` is still TCB. Hostile on-wire peers beyond this probe are out of product scope. |

## Trust boundaries

```
[ host / QEMU / CI ]  --trusted config--  [ virt guest ]
                                              |
                                         EL1 kernel  ← TCB today
                                              |
                                         EL0 first mile + standing  ← entered / stood / left
                                              |                       (not a POSIX user)
                                         EL0 isolation    ← Planned / non-claim (ADR-047)
```

- **EL1 now.** Page tables distinguish *execute* (RO+X text vs RW+NX data/stacks/heap vs Device XN), *write* (AP[2] on text), and *presence* (guard holes).
- **EL0 first mile + user TTBR0 + ASID TLB + standing + standing-as-normal + TTBR1 first cut + EL1 high-VA fetch + identity-tear first cut + identity `.text` range tear + live `.text` tear + identity `.rodata` tear + EL0 entry without `VMALLE1` + lower-EL IRQ while standing + IRQ-unmasked default ERET + lower-EL FIQ while standing + SError park honesty + PAN ID-field + SVC ABI + libctos CRT + guest PT_LOAD loader, isolation Planned.** Lower-EL AArch64 **sync** is taken (SVC / IABORT / DABORT). Lower-EL **IRQ** while standing is taken and returns to EL0 (`el0: irq`); default standing/task `ERET` clears I (`el0: irq-default`); lower-EL **FIQ** while standing is taken when FIQEn is armed (`el0: fiq`); SError park honesty (`el0: serror-park`); ADR-045 A-clear + QMP attempt (inject failed). Short non-standing trampoline probes stay IRQ-masked. Dual ASID without `TLBI VMALLE1` is a probed mile (`asid: ok`). Standing/EL0 trampoline also switches without `TLBI VMALLE1` after identity `.data`/heap tear (`el0: no-vmalle1`). Standing dual-SVC flips `is_active()`. A standing **task** runs a loaded image until `exit` (`el0: task-ok`); unexpected sync restores fail-closed (`el0: restore-fail`). TTBR1 private page is EL1-only (`ttbr1: ok`). EL1 can fetch a real path from the high RAM alias (`ttbr1: el1 exec`); `VBAR_EL1` is that alias (`ttbr1: vbar`). A 16 KiB dedicated identity text range is unmapped (`ident: range`); live identity `.text` after `_start` is unmapped after a vtable rewrite (`ident: reloc` / `ident: live`); identity `.rodata` is unmapped (`ident: rodata`); identity `.data`/stacks are unmapped (`ident: data`); identity heap is unmapped (`ident: heap`); `_start` stays (`ident: start-stay`); leftover identity RAM after the heap is torn (`ident: ram`). `ID_AA64MMFR1_EL1.PAN` is 0 on `-cpu cortex-a57` (`pan: absent`). A documented SVC ABI (`svc: ok`) is an ABI mile. A linked `libctos` hello (`libctos: ok`) is a CRT mile. A guest `PT_LOAD` (`loader: ok`) is a loader mile. Standing-as-normal (`el0: task-ok`) is a mile; product freestanding hosting is Verified separately under ADR-048/052. Umbrella *isolation* stays Planned/non-claim under ADR-047 decisions (PAN enable non-goal on default a57; taken SError deferred; never yank `_start`; leftover RAM torn via ADR-049; umbrella still Planned/non-claim). A caught load of `.data` or of `TTBR1_PRIV` is that mile, not “EL0 isolated.”
- **Repo vs guest.** “No secrets in repo” is a host boundary. It does not harden the UART.

## Non-goals

- Not a **certified** secure OS (no Common Criteria, no PSA, no “hardened”).
- Not **side-channel complete** (no cache/timing/Spectre story; QEMU TCG is the wrong lab).
- Not a **product “the kernel is W^X”** sentence. The identity image on this virt guest is RO+X / RW+NX with WXN ([ADR-015](../03-adr/ADR-015-ro-nx-text-data.md)). Future mappings are not automatically covered. Guard pages remain **holes**.
- Not **ASAN / canaries / heap-stack guards**. Coop worker stacks have no unmapped holes. Overflow there is still image-adjacent PXN RAM.
- Not secure boot, PAN enable on default cortex-a57 (non-goal, [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)), or a fully torn-down identity map (ADR-020/025/037/038/049 unmap live `.text` / `.rodata` / `.data`+stacks / heap / leftover frame RAM; `_start` stays at `0x4008_0000` by decision).

## Mitigations mapped to probes

| Mitigation | Probe | Ledger |
| --- | --- | --- |
| Threat-model v1.23 written | Read this file | Verified (file + review). Still no “secure OS”. |
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
| EL0 entry without `VMALLE1` | Serial `el0: no-vmalle1`; `src/exception.rs` has no `tlbi vmalle1` | Trampoline cut ([ADR-039](../03-adr/ADR-039-el0-entry-without-vmalle1.md)). Not isolation. |
| Lower-EL IRQ while standing | Serial `el0: irq` | Taken IRQ from EL0 returns to standing ([ADR-040](../03-adr/ADR-040-lower-el-irq-standing.md)). Not isolation. |
| IRQ-unmasked default ERET | Serial `el0: irq-default` | Default standing/task `ERET` clears I ([ADR-041](../03-adr/ADR-041-irq-unmasked-default-eret.md)). Short probes stay masked. Not isolation. |
| Lower-EL FIQ while standing | Serial `el0: fiq` | Taken FIQ from EL0 returns to standing ([ADR-043](../03-adr/ADR-043-lower-el-fiq-serror.md)). Not isolation. |
| SError park honesty | Serial `el0: serror-park` | No working SError inject on this virt guest ([ADR-043](../03-adr/ADR-043-lower-el-fiq-serror.md) / [ADR-045](../03-adr/ADR-045-taken-serror-qmp.md)). Taken path deferred/non-goal ([ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). |
| Taken SError QMP attempt | Serial `el0: serror-arm` + QMP log | Guest A-clear path ([ADR-045](../03-adr/ADR-045-taken-serror-qmp.md)); `inject-nmi` fails on QEMU 10 virt+a57. |
| Identity `_start` stay | Serial `ident: start-stay` | Honesty ([ADR-042](../03-adr/ADR-042-isolation-leftover-wrap.md) / [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). Not isolation. |
| Leftover identity RAM tear | Serial `ident: ram` / `ident: ram-fault` / `ident: ram-high` | Optional mile ([ADR-049](../03-adr/ADR-049-identity-ram-tear.md)). High twin stays. Not isolation. |
| Standing EL0 context | Serial `el0: standing` / `el0: restored`; `is_active()` flips | Specific mile. Not POSIX. Not isolation. |
| Standing EL0 as normal mode | Serial `el0: task-enter` / `el0: task-active` / `el0: task-exit` / `el0: task-restored` / `el0: restore-fail` / `el0: task-ok` | Loaded image until `exit`. Fail-closed restore. Not isolation. |
| TTBR1 private page | Serial `ttbr1: el1` / `ttbr1: no el0` / `ttbr1: ok` | First cut ([ADR-016](../03-adr/ADR-016-ttbr1-private-page.md)). |
| EL1 fetch from TTBR1 high VA | Serial `ttbr1: el1 exec` / `ttbr1: vbar` | Exec mile ([ADR-017](../03-adr/ADR-017-ttbr1-high-el1-exec.md)). |
| Identity-tear first cut | Serial `ident: split` / `ident: fault` / `ident: high` / `ident: no el0` / `ident: ok` | First cut ([ADR-018](../03-adr/ADR-018-identity-teardown-first-cut.md)). Yank `_start` decided never; leftover RAM torn ([ADR-049](../03-adr/ADR-049-identity-ram-tear.md)). |
| Identity text range tear | Serial `ident: jump` / `ident: range` / `ident: text` | Range cut ([ADR-019](../03-adr/ADR-019-identity-text-range-tear.md)). |
| High-VA vtable rewrite + live `.text` tear | Serial `ident: reloc` / `ident: live` | Live `.text` cut ([ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md)). |
| Identity `.rodata` tear | Serial `ident: rodata` / `ident: rodata-fault` / `ident: rodata-high` | `.rodata` cut ([ADR-025](../03-adr/ADR-025-identity-rodata-tear.md)). |
| Identity `.data` / stack tear | Serial `ident: data` / `ident: data-fault` / `ident: data-high` | `.data`/stacks cut ([ADR-037](../03-adr/ADR-037-identity-data-tear.md)). |
| Identity heap tear | Serial `ident: heap-reloc` / `ident: heap` / `ident: heap-fault` / `ident: heap-high` | Heap cut ([ADR-038](../03-adr/ADR-038-identity-heap-tear.md)). |
| PAN ID field | Serial `pan: id=` / `pan: absent` | Capability probe ([ADR-026](../03-adr/ADR-026-pan-capability.md)). Enable non-goal on default probe CPU ([ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). |
| SVC ABI (`exit` / `uart_write` / `yield`) | Serial `svc: ok`; user buffer + kernel-`.data` / TTBR1-alias reject | ABI mile ([ADR-021](../03-adr/ADR-021-svc-syscall-abi.md)). Not app hosting. |
| `libctos` CRT | Serial `libctos: hi` / `libctos: ok` / `libctos: linked` | CRT mile ([ADR-022](../03-adr/ADR-022-libctos-crt.md)). Not app hosting. |
| Guest ELF PT_LOAD loader | Serial `loader: mapped` / `loader: ok` | Loader mile ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). Not a Linux ABI. Not app hosting. |
| Thin VFS + memfs | Serial `fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` | memfs mile ([ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)). Not POSIX. Not app hosting. |
| virtio-net ARP (N1) + ICMP ping (N2) + UDP (N3) + thin TCP (N5) | Serial `net: virtio` / `net: mac` / `net: tx` / `net: rx` / `net: ok` + `net: icmp-tx` / `net: icmp-rx` / `net: ping-ok` + `net: udp-tx` / `net: udp-rx` / `net: udp-ok` + `net: tcp-syn` / `net: tcp-est` / `net: tcp-tx` / `net: tcp-rx` / `net: tcp-ok` | [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) / [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) / [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) / [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md). ARP + ICMP + one UDP + thin TCP vs QEMU SLIRP/`guestfwd` only. Not BSD sockets / TCP product. Host `-netdev` alone is not the probe. |
| EL0 net SVC sample (N4) | Serial `libctos: net-hi` / `libctos: net-mac` / `libctos: net-ok` / `netdemo: ok` | [ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md). SVCs only; kernel owns NIC. Not sockets. |
| EL0 UDP DNS SVC sample (N3.x) | Serial `libctos: udp-hi` / `libctos: udp-ok` / `udpdemo: ok` | [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md). `net_udp_dns` only; DNS is probe bait. Not sockets. |
| virtio-blk + FAT16 | Serial `blk: ok` / `fat: ok` / `fat: write` / `fat: create` / `fat: grow` / `fat: mkdir` / `fat: readdir` / `fat: entries` / `fat: delete`; VFS `/probe` | block + FAT mile ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)) + write depth ([ADR-050](../03-adr/ADR-050-fat16-write.md)) + multi-cluster grow ([ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md)) + mkdir ([ADR-073](../03-adr/ADR-073-fat16-mkdir.md)) + readdir ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)) + delete ([ADR-057](../03-adr/ADR-057-fat16-delete.md)). Host `-drive` alone is not the probe. Not POSIX `getdents`/`unlink`/`mkdir`/write API. |
| OS/app slot first cut | Serial `slot: fat` / `slot: mapped` / `slot: ok` | Slot mile ([ADR-030](../03-adr/ADR-030-os-app-slots.md)). A2–A4 load FAT (no embed). |
| A9 cross-update | Host smoke: same app `sha256` on this OS and `ba6541c` | Leftover ([ADR-032](../03-adr/ADR-032-track-a-leftovers.md)). Not “apps update independently.” |
| RO+NX text/data | Serial `ro: ok`; execute-from-`.data` + write-to-RO-text | Verified: 2026-09-11 cloud `qemu-smoke` (honesty ledger). |

## Claim gate

A PR may say “threat-model v1.22 exists” after a file read. It may say “heap NX” / “identity image is W^X on this virt guest” only when the honesty ledger has a matching **Verified** QEMU probe. It may say “linker-stack guard faults” only with a matching translation-abort probe. It may say “EL0 entered and returned,” “EL0 cannot execute kernel data,” “EL0 cannot read kernel `.data`,” “standing EL0 context,” “ASID isolation,” “TTBR1 private page,” “EL1 fetched from a TTBR1 high VA,” “one identity text page was unmapped,” “a 16 KiB dedicated identity text range was unmapped,” “rustc vtables were rewritten to high aliases,” “live identity `.text` after the boot stub was unmapped,” “identity `.rodata` was unmapped,” “identity `.data`/stacks were unmapped,” “`ID_AA64MMFR1_EL1.PAN` is 0 on `-cpu cortex-a57`,” “EL0 issued the documented SVC ABI,” “a program linked against libctos issued that ABI,” “the guest mapped PT_LOAD and ERETed to e_entry,” or “a loaded image stood at EL0 until exit with fail-closed restore,” or “the guest created, wrote, and read a named in-RAM file,” or “the guest programmed virtio-blk and read a known FAT16 file through the thin VFS,” or “the guest wrote FAT16 bytes through the thin VFS,” or “the guest listed FAT16 root directory entries through the thin VFS,” or “the guest loaded a separate app ELF from FAT `/hello` into user TTBR0,” or “the same published app ELF loaded on this OS and on `ba6541c`,” or “identity heap was unmapped,” or “standing/EL0 trampoline switches without `TLBI VMALLE1`,” or “taken lower-EL IRQ while standing returned to EL0,” or “default standing/task ERET enters with IRQ unmasked,” or “`_start` stays mapped while live image ranges are torn” only for the mile that actually passed.

It may **not** say “secure OS,” “hardened,” “EL0 works,” “EL0 isolated,” “the kernel moved,” “immutable OS,” Linux app host, POSIX host, or container host. It **may** say the freestanding product app-hosting claim is **Verified under ADR-048** when the ledger product row is Verified ([ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md)). Scoped RO is [immutability.md](immutability.md). Unprobed stays **Unknown**. https://ctos.artof.link HTTPS is **Verified** (2026-09-11 after #30; see the [honesty ledger](honesty-ledger.md)).

### Taken SError research + QMP attempt (v1.25)

Research ([ADR-044](../03-adr/ADR-044-taken-serror-research.md)); implementation attempt ([ADR-045](../03-adr/ADR-045-taken-serror-qmp.md)): guest A-clear standing path + host QMP `inject-nmi`. On QEMU 10 virt+`-cpu cortex-a57`, QMP returns `machine does not provide NMIs` (no `ARM_CPU_SERROR` via nmi; FEAT_NMI needs a different CPU/GIC). Park marker remains Verified.

### Isolation leftovers decisions + app-hosting claim gate (v1.26)

[ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md): taken SError = **deferred/non-goal** on this smoke machine (A-clear dormant prep); PAN enable = **non-goal** on default cortex-a57; **never yank `_start`**; leftover identity RAM torn via [ADR-049](../03-adr/ADR-049-identity-ram-tear.md) (`ident: ram*`); umbrella “EL0 isolated” stays **Planned/non-claim**. [ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) (at v1.26): A1–A9 first cuts ≠ product “app hosting done”; claim stayed Planned until criteria Met + sponsor accept.

### Product app-hosting claim Verified (v1.29)

[ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) criteria **1–5 Met**; sponsor accept [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md). Product freestanding “apps run independently / hosting done” is **Verified**. Still **not** Linux userspace, POSIX, glibc/musl, guest OCI/Docker/k8s, “EL0 isolated,” PAN enable, taken SError Verified, immutable-OS marketing, or OTA

### Isolation-gap locks (v1.30)

[ADR-053](../03-adr/ADR-053-taken-serror-hard-stop.md): taken lower-EL SError **hard-stopped** on QEMU 10 virt+cortex-a57 (dated 2026-09-14 re-probe: `inject-nmi` / HMP `nmi` → `machine does not provide NMIs`); park Verified; A-clear dormant prep. [ADR-054](../03-adr/ADR-054-pan-enable-lock.md): PAN enable **locked** non-goal on default cortex-a57; explicit reopen gate (sponsor CPU + FEAT_PAN + enable+fault ADR). [ADR-055](../03-adr/ADR-055-el0-isolated-checklist.md): umbrella “EL0 isolated” **non-claim until checklist** — Verified miles listed; unmet blockers include PAN enable, taken SError, never-yank `_start`. Still **not** Guest Linux / containers / immutable-OS marketing..

### FAT16 readdir (v1.31)

[ADR-056](../03-adr/ADR-056-fat16-readdir.md): guest lists FAT16 **root** dirents as thin-VFS paths (`fat: readdir` / `fat: entries`). Not POSIX `getdents` / `opendir`. Keep write / `/hello` / `slot: ok`. Still **not** Linux/containers/“EL0 isolated.”

### FAT16 delete (v1.32)

[ADR-057](../03-adr/ADR-057-fat16-delete.md): guest deletes a FAT16 **root** file via thin-VFS `unlink` (`fat: delete`). Not POSIX `unlink` / `remove`. Keep write / readdir / `/hello` / `slot: ok`. Still **not** Linux/containers/“EL0 isolated.”

### VFS prefix mounts (v1.33)

[ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md): guest routes path **prefixes** through a thin VFS mount table (`vfs: mount` / `vfs: mounts`). Not POSIX `mount(2)` / Linux vfsmount. Keep memfs / FAT write / readdir / delete / `/hello` / `slot: ok`. Still **not** Linux/containers/“EL0 isolated.”


### Freestanding sample catalog (v1.34)

Two freestanding EL0 samples on FAT: `/hello` (`hello-libctos`, UART+yield) and `/fsdemo` (`fs-libctos`, libctos VFS create/open/read/write/close on `/memdemo`). Catalog docs list both. Not POSIX. Not `getdents`. Not a reopen of product app hosting.

### Sponsor isolation leftovers closure (v1.35)

[ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md): single sponsor-facing table for taken SError / PAN enable / umbrella “EL0 isolated” / never-yank `_start` — **locks + reopen gates only**. Cites ADR-047 / ADR-053 / ADR-054 / ADR-055 / #98. Does **not** invent Verified. Does **not** reopen Guest Linux / containers / immutable-OS marketing.

### Freestanding FAT-via-VFS sample (v1.36)

Three freestanding EL0 samples on FAT: `/hello`, `/fsdemo` (memfs `/memdemo`), and `/fatdemo` (`fat-libctos`, open/read FAT `/probe` via thin VFS `/` → FAT16). Catalog docs list all three ([ADR-061](../03-adr/ADR-061-fat-libctos-sample.md)). Not POSIX. Not `getdents`. Not a reopen of product app hosting.
### FAT16 multi-cluster grow (v1.39)

[ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md): guest grows a FAT16 root file across a cluster boundary (`fat: grow`); EL0 `fat-libctos` chunked SVC write/read-back (`libctos: fat-grow`). Not POSIX write API. Not exFAT. Keep write / readdir / delete / `/hello` / `slot: ok`. Still **not** Linux/containers/“EL0 isolated.” Track N untouched.

### FAT vs memfs read CNTPCT (v1.40)

[ADR-065](../03-adr/ADR-065-fat-memfs-read-cntpct.md): same-boot CNTPCT pair around FAT `/probe` read vs memfs `/mrprobe` read (`perf: fat-read` / `perf: memfs-read` / `perf: fs-read-delta`). Measure-first lab ticks only — not a latency SLA, percent, or product KPI. Keep ADR-051 write pair / `fat: ok` / Track N untouched. Still **not** “secure.”

### Freestanding cooperative-yield sample (v1.37)

[ADR-062](../03-adr/ADR-062-yield-libctos-sample.md): fourth freestanding EL0 sample `yield-libctos` on FAT `/yldemo`. Several cooperative `yield_now()` rounds (`libctos: yld-hi` / `libctos: beat` / `libctos: yld-ok`). Not preemption. Not multi-task EL0. Not a process table. Does not reopen ADR-048/052.


### Track N network foundation scope (v1.38)

[ADR-063](../03-adr/ADR-063-network-foundation-scope.md): new Track N (network), separate from A/B. N0 = docs scope. Non-goals locked: TCP/UDP/sockets/DHCP/DNS product, Wi‑Fi, virtio-pci-only foundation, Linux net stack, “has networking” marketing, EL0 net ABI before deeper net miles.

### Track N virtio-net first frame (v1.41)

[ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md): N1 discovers virtio-net, programs RX+TX, TX ARP for `10.0.2.2`, RX SLIRP reply (`net: virtio` / `net: mac` / `net: tx` / `net: rx` / `net: ok`). Host `-netdev user,id=net0 -device virtio-net-device,netdev=net0`. I/O surface: virtio-blk **and** virtio-net are programmed DMA masters. Still not TCP/UDP/sockets/DHCP/DNS/Wi‑Fi/“has networking.”

### Track N virtio-net ICMP ping (v1.42)

[ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md): after N1 ARP, guest TX ICMP echo request to `10.0.2.2` and RX echo reply (`net: icmp-tx` / `net: icmp-rx` / `net: ping-ok`). QEMU args unchanged. Kernel-path only. Still not TCP/UDP/sockets/DHCP/DNS/Wi‑Fi/“has networking.”

### Track N EL0 net SVC sample (v1.43)

[ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md): `SYS_NET_MAC` (24) + `SYS_NET_PING` (25); freestanding `net-libctos` on FAT `/netdemo` (`libctos: net-hi` / `libctos: net-mac` / `libctos: net-ok` / `netdemo: ok`). Kernel still owns virtio-net. Still not TCP product / sockets / DHCP/DNS product / Wi‑Fi / “has networking.”

### Net-ping CNTPCT (v1.44)

[ADR-069](../03-adr/ADR-069-net-ping-cntpct.md): CNTPCT around EL0 `net_ping` quiet ARP+ICMP (`perf: net-ping ticks=<n>`). Measure-first lab ticks only — not a latency SLA, percent, or product KPI. Smoke greps the marker **prefix** (tick values vary). Keep N1/N2/N4 / FAT / slot. Still **not** TCP/UDP/sockets/“has networking.” Still **not** “secure.”

### Track N UDP transport (v1.45)

[ADR-070](../03-adr/ADR-070-n3-udp-transport.md): after N2 ICMP, guest TX one UDP DNS query to SLIRP `10.0.2.3:53` and RX a matching DNS response (`net: udp-tx` / `net: udp-rx` / `net: udp-ok`). QEMU args unchanged. Kernel-path only this mile. DNS is **probe bait**, not a guest DNS product. Still not BSD sockets / TCP product / DHCP/DNS product / Wi‑Fi / “has networking / sockets OS.” Still **not** “secure.”

### Track N EL0 UDP DNS SVC (v1.46)

[ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md): `SYS_NET_UDP_DNS` (26); freestanding `udp-libctos` on FAT `/udpdemo` (`libctos: udp-hi` / `libctos: udp-ok` / `udpdemo: ok`). Quiet kernel-path ARP + UDP DNS to SLIRP `10.0.2.3:53` (same bait as ADR-070). Kernel still owns virtio-net. Still not BSD sockets / TCP product / DHCP/DNS product / Wi‑Fi / “has networking.” Still **not** “secure.”

### UDP DNS path CNTPCT (v1.47)

[ADR-072](../03-adr/ADR-072-udp-dns-cntpct.md): CNTPCT around EL0 `net_udp_dns` quiet ARP+UDP DNS (`perf: udp-dns ticks=<n>`). Measure-first lab ticks only — not a latency SLA, percent, or product KPI. Smoke greps the marker **prefix** (tick values vary). Keep N1/N2/N3/N3.x/N4 / ADR-069 / FAT / slot. Still **not** TCP/sockets/DNS product/“has networking.” Still **not** “secure.”


### FAT16 mkdir (v1.48)

[ADR-073](../03-adr/ADR-073-fat16-mkdir.md): guest creates a FAT16 root directory through the thin VFS (`fat: mkdir`); dirent + empty cluster with `.` / `..`. Not POSIX `mkdir`. Not a directory tree. Not exFAT. No new SVC this mile (EL0 follow-up). Keep write / readdir / delete / grow / `/hello` / `slot: ok` / Track N markers. Still **not** Linux/containers/“EL0 isolated.” Track N / TCP untouched.

### Thin TCP N5 (v1.49)

[ADR-074](../03-adr/ADR-074-n5-thin-tcp.md): after N3 UDP, guest active-opens TCP to QEMU `guestfwd` `10.0.2.4:7`, sends one payload `ctos-tcp\n`, requires matching echo (`net: tcp-syn` / `net: tcp-est` / `net: tcp-tx` / `net: tcp-rx` / `net: tcp-ok`). QEMU `-netdev` gains `guestfwd=tcp:10.0.2.4:7-cmd:…/tcp-echo-stdio.sh`. Kernel-path only this mile. One connection; no listen/accept; no EL0 TCP SVC. Still not BSD sockets / TCP product / TLS/HTTP / DHCP/DNS product / Wi‑Fi / “has networking / sockets OS.” Still **not** “secure.”
