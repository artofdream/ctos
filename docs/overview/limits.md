# Drawbacks / limits

What this project **is not**, and what is still unfinished. Pair with [KPIs / how we measure](measure.md) and [product vision](../01-vision/product-vision.md).

## Learning kernel, not a product

ctos is a research / teaching AArch64 kernel. It is not production-ready, not a desktop, not POSIX, not a container host (**non-goal**, [ADR-029](../03-adr/ADR-029-containers-nongoal.md)), and not a “secure OS.” Do not treat a green QEMU smoke as certification. Rust helps discipline; it does not remove these limits ([Why Rust](advantages.md#why-rust-for-this-kernel)).

## Boot contract still starts at `0x4008_0000`

QEMU `-kernel` and `_start` stay at `0x4008_0000`. High-address work (kernel page-table alias, live code / `.rodata` / `.data` / heap / leftover frame-RAM tears) does **not** mean the kernel moved. Leftover identity RAM after the heap is unmapped (`ident: ram*`, [ADR-049](../03-adr/ADR-049-identity-ram-tear.md)); smoke **rejects** `ident: ram-stay`. `_start` stays mapped by decision (`ident: start-stay`, [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). Do not claim full identity teardown. See [el0.md](../framework/el0.md).

## Privileged Access Never — enable is a non-goal on `cortex-a57`

**PAN** is a hardware feature that would stop the kernel from casually reading user memory. Default probe CPU is `-cpu cortex-a57` (ARMv8.0). The ID-field probe is Verified (`pan: absent`). **PAN enable is a locked non-goal on that default probe CPU** ([ADR-054](../03-adr/ADR-054-pan-enable-lock.md) / [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md)). Do not claim PAN. Do not silently switch `-cpu`. Reopen gate: sponsor-approved default CPU + FEAT_PAN + enable+fault ADR.

## No network; disk is virtio-blk + FAT16 only

There is no NIC driver and no virtio-net. A7 programs virtio-mmio block and reads a host-built FAT16 image through the thin VFS; ADR-050 adds a guest FAT16 write / small create mile; ADR-051 adds a same-boot FAT vs memfs write CNTPCT pair (lab measurement, not a bench) ([Filesystem](filesystem.md), [KPIs](measure.md)). That is not a general DMA API, not virtio-pci, not POSIX, and not a Linux rootfs. Input on virt is serial receive. Timer is the virt interrupt controller plus the generic timer.

## No real userspace apps

The standing user-mode stub is a mile, not an application runtime. No ELF loader for third-party programs, no libc, no extra CPUs, no GPU. Rebuild recipes: [What can run today](what-can-run.md) — three freestanding samples on FAT (`/hello`, `/fsdemo`, `/fatdemo`). How to extend the kernel (not port POSIX): [Building or porting](porting.md). OS vs app slot has an A9 first cut (FAT `/hello`) plus leftover cross-update on `ba6541c` (Verified miles). Product freestanding app hosting is **Verified** under [ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) / [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md) (not Linux/POSIX). Isolation leftovers stay locked non-claim ([ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md)).

## Docs URL

`https://ctos.artof.link` **serves this book** (HTTPS **Verified** after #30 — [website.md](../website.md)). A trailing-slash `github.io/ctos/` path 404’d on the 2026-09-11 probe; use the custom domain. Publishing mechanics can still lag a *new* commit until the next `main` Pages deploy.

## Immutability is not absolute

Read-only code / not-executable data, and torn identity ranges (including leftover frame RAM), are **scoped** probes. Do not upgrade them to “immutable kernel,” “W^X everywhere,” or “OS updates without touching apps.” The A9 OS vs app slot **first cut** exists; the freestanding product hosting claim is **Verified** ([ADR-048](../03-adr/ADR-048-app-hosting-claim-criteria.md) / [ADR-052](../03-adr/ADR-052-sponsor-accept-app-hosting.md)). Not OTA or containers. Details: [Advantages — Immutability](advantages.md#immutability).

## Other honest gaps

- Umbrella user-mode isolation: **Planned / non-claim until checklist** ([ADR-055](../03-adr/ADR-055-el0-isolated-checklist.md) / [ADR-047](../03-adr/ADR-047-isolation-leftovers-decisions.md); sponsor closure [ADR-060](../03-adr/ADR-060-isolation-leftovers-closure-checklist.md); PAN enable locked ([ADR-054](../03-adr/ADR-054-pan-enable-lock.md)); `_start` stays; taken SError hard-stopped ([ADR-053](../03-adr/ADR-053-taken-serror-hard-stop.md)); leftover RAM torn via ADR-049 — still not “EL0 isolated”).
- x86_64 is not a primary path.
- Raspberry Pi / other boards: unprobed.
- CI on GitHub is a separate ledger row from a cloud-VM `qemu-smoke`.
