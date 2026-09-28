# Drawbacks / limits

What this project **is not**, and what is still unfinished. Pair with [KPIs / how we measure](measure.md) and [product vision](../01-vision/product-vision.md).

## Learning kernel, not a product

ctos is a research / teaching AArch64 kernel. It is not production-ready, not a desktop, not POSIX, not a container host (**non-goal**, [ADR-029](../03-adr/ADR-029-containers-nongoal.md)), and not a “secure OS.” Do not treat a green QEMU smoke as certification. Rust helps discipline; it does not remove these limits ([Why Rust](advantages.md#why-rust-for-this-kernel)).

## Boot contract still starts at `0x4008_0000`

QEMU `-kernel` and `_start` stay at `0x4008_0000`. High-address work (kernel page-table alias, live code / `.rodata` / `.data` / heap / leftover frame-RAM tears) does **not** mean the kernel moved. Leftover identity RAM after the heap is unmapped (`ident: ram*`, [ADR-049](../03-adr/ADR-049-identity-ram-tear.md)); smoke **rejects** `ident: ram-stay`. `_start` stays mapped by decision (`ident: start-stay`, [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). Do not claim full identity teardown. See [el0.md](../framework/el0.md).

## Privileged Access Never — enable Verified on cortex-a76 (ADR-080)

**PAN** stops the kernel from casually reading user-accessible pages when PSTATE.PAN is set. Default smoke CPU is `-cpu cortex-a76` ([ADR-079](../03-adr/ADR-079-pan-cpu-reopen.md) / [#125](https://github.com/artofdream/ctos/issues/125)): ID-field Verified (`pan: present`) and enable + EL1-vs-EL0 fault Verified (`pan: enabled` / `pan: el1-fault`, [ADR-080](../03-adr/ADR-080-pan-enable-fault.md)). Historical `-cpu cortex-a57` remains `pan: absent` ([ADR-026](../03-adr/ADR-026-pan-capability.md)). Say “EL0 isolated” only as the accepted ADR-091 sentence (below). Do not silent `-cpu`.

## Network is virtio-net ARP + ICMP + minimal UDP + thin TCP (+ tiny EL0 net/UDP SVCs); disk is virtio-blk + FAT16

N1 programs virtio-mmio **virtio-net** enough to TX/RX one raw Ethernet frame (ARP vs QEMU SLIRP gateway) — [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md). N2 adds a kernel-path **ICMP echo** to `10.0.2.2` (`net: ping-ok`) — [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md). N3 adds a kernel-path **UDP datagram** to SLIRP DNS `10.0.2.3:53` (`net: udp-ok`) — [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) (DNS is probe bait, not a DNS product). N5 adds a kernel-path **thin TCP** active-open + one payload echo via QEMU `guestfwd` `10.0.2.4:7` (`net: tcp-ok`) — [ADR-074](../03-adr/ADR-074-n5-thin-tcp.md) (not a sockets product). N4 adds a tiny EL0 net SVC (`net_mac` / `net_ping`) and freestanding `/netdemo` — [ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md). N3.x adds EL0 `net_udp_dns` and `/udpdemo` — [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md) / [track-n.md](../04-roadmap/track-n.md). That is **not** BSD sockets, TCP product, DHCP/DNS product, Wi‑Fi, or “has networking / sockets OS.” A7 programs virtio-mmio block and reads a host-built FAT16 image through the thin VFS; ADR-050 adds a guest FAT16 write / small create mile; ADR-051 / ADR-065 add same-boot FAT vs memfs write and read CNTPCT pairs (lab measurements, not benches) ([Filesystem](filesystem.md), [KPIs](measure.md)). That is not a general DMA API, not virtio-pci, not POSIX, and not a Linux rootfs. Input on virt is serial receive. Timer is the virt interrupt controller plus the generic timer.

## No real userspace apps

The standing user-mode stub is a mile, not an application runtime. No ELF loader for third-party programs, no libc, no extra CPUs, no GPU. Rebuild recipes: [What can run today](what-can-run.md) — four freestanding samples on FAT (`/hello`, `/fsdemo`, `/fatdemo`, `/yldemo`). How to extend the kernel (not port POSIX): [Building or porting](porting.md). OS vs app slot has an A9 first cut (FAT `/hello`) plus leftover cross-update on `ba6541c` (Verified miles). Product freestanding app hosting is **Verified** under [ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) / [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md) (not Linux/POSIX). Umbrella “EL0 isolated” is **Verified** only as the accepted [ADR-091](../03-adr/ADR-091-el0-isolated-accept-draft.md) sentence (M6); the other isolation locks stay ([ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md)).

## Docs URL

`https://ctos.artof.link` **serves this book** (HTTPS **Verified** after #30 — [website.md](../website.md)). A trailing-slash `github.io/ctos/` path 404’d on the 2026-09-11 probe; use the custom domain. Publishing mechanics can still lag a *new* commit until the next `main` Pages deploy.

## Immutability is not absolute

Read-only code / not-executable data, and torn identity ranges (including leftover frame RAM), are **scoped** probes. Do not upgrade them to “immutable kernel,” “W^X everywhere,” or “OS updates without touching apps.” The A9 OS vs app slot **first cut** exists; the freestanding product hosting claim is **Verified** ([ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) / [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md)). Not OTA or containers. Details: [Advantages — Immutability](advantages.md#immutability).

## Other honest gaps

- Umbrella user-mode isolation: **Verified** only as the accepted [ADR-091](../03-adr/ADR-091-el0-isolated-accept-draft.md) sentence (M6, 2026-09-28; checklist [ADR-055](../03-adr/ADR-055-el0-isolated-checklist.md) all Met; sponsor closure [ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md)). The sentence, verbatim:
  > “On the ctos default smoke machine (QEMU `virt`, `-cpu cortex-a76`, TCG, one core), EL0 is isolated from the kernel at the architectural page-table / exception-level boundary: each EL0 task runs on its own TTBR0 with its own ASID; EL0 faults when it reads, writes or executes kernel memory; in every translation table (kernel, user and ASID-B TTBR0, and TTBR1) the only EL0-reachable pages are five allowlisted user slots backed by non-kernel frames, checked fail-closed while a user app runs and at rest; PAN is enabled and EL1 faults on EL0 memory; lower-EL IRQ and FIQ are taken while EL0 stands; taken lower-EL SError is evidenced by the ADR-090 evidence class; the only identity (VA = PA) mapping left in any TTBR0 is the documented EL1-only `_start` stub page (ADR-089); and a system call copies from or to an EL0 pointer only if EL0 itself may read (for a copy from EL0) or write (for a copy to EL0) every page in the range. It is not a speculative-execution or side-channel claim, not a real-hardware claim, and not a certification.”
  Limits that stay with it: one core; the TCG default smoke machine; not speculative-execution / side-channel; not real hardware; not a certification; EL0 can read its own code bytes ([ADR-094](../03-adr/ADR-094-syscall-pointer-el0-permission.md)) — [ADR-097](../03-adr/ADR-097-el0-execute-only-app-text.md) proposes execute-only app text, pending a sponsor rev 4 accept; no multi-core race story. Also outside it: task-vs-task isolation beyond ASIDs, DMA / IOMMU / EL2, and taken SError on stock QEMU (still parks, [ADR-053](../03-adr/ADR-053-taken-serror-hard-stop.md)). `_start` stays ([ADR-089](../03-adr/ADR-089-start-stub-identity-exception.md)).
- x86_64 is not a primary path.
- Raspberry Pi / other boards: unprobed.
- CI on GitHub is a separate ledger row from a cloud-VM `qemu-smoke`.
