# Drawbacks / limits

What this project **is not**, and what is still unfinished. Pair with [KPIs / how we measure](measure.md) and [product vision](../01-vision/product-vision.md).

## Learning kernel, not a product

ctos is a research / teaching AArch64 kernel. It is not production-ready, not a desktop, not POSIX, not a container host, and not a “secure OS.” Do not treat a green QEMU smoke as certification.

## Boot contract still starts at `0x4008_0000`

QEMU `-kernel` and `_start` stay at `0x4008_0000`. High-address work (kernel page-table alias, live code tear after a vtable rewrite) does **not** mean the kernel moved. Some data can still be identity-mapped (virtual address equals physical). Full identity teardown is **Planned**. See [el0.md](../framework/el0.md) and [ADR-020](../03-adr/ADR-020-identity-fnptr-reloc.md).

## Privileged Access Never unclaimed on `cortex-a57`

**PAN** is a hardware feature that would stop the kernel from casually reading user memory. Default probe CPU is `-cpu cortex-a57` (ARMv8.0). Do not claim PAN. The [ledger](../framework/honesty-ledger.md) row stays **Planned** until the CPU reports the feature **and** an access fault is probed. Do not silently switch `-cpu`.

## No network; disk is virtio-blk + FAT16 only

There is no NIC driver and no virtio-net. A7 programs virtio-mmio block and reads a host-built FAT16 image through the thin VFS ([Filesystem](filesystem.md)). That is not a general DMA API, not virtio-pci, and not a Linux rootfs. Input on virt is serial receive. Timer is the virt interrupt controller plus the generic timer.

## No real userspace apps

The standing user-mode stub is a mile, not an application runtime. No ELF loader for third-party programs, no libc, no extra CPUs, no GPU. Concrete examples: [What can run today](what-can-run.md). How to extend the kernel (not port POSIX): [Building or porting](porting.md).

## Docs URL

`https://ctos.artof.link` **serves this book** (HTTPS **Verified** after #30 — [website.md](../website.md)). A trailing-slash `github.io/ctos/` path 404’d on the 2026-09-11 probe; use the custom domain. Publishing mechanics can still lag a *new* commit until the next `main` Pages deploy.

## Immutability is not absolute

Read-only code / not-executable data, and a torn identity code range, are **scoped** probes. Do not upgrade them to “immutable kernel,” “W^X everywhere,” or “OS updates without touching apps.” An OS slot vs app slot waits on Track A and is not OTA or containers. Details: [Advantages — Immutability](advantages.md#immutability).

## Other honest gaps

- Umbrella user-mode isolation: **Planned** (PAN + full identity teardown still missing).
- x86_64 is not a primary path.
- Raspberry Pi / other boards: unprobed.
- CI on GitHub is a separate ledger row from a cloud-VM `qemu-smoke`.
