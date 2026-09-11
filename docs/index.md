# ctos docs

Learning-kernel documentation for [ctos](https://github.com/artofdream/ctos): a minimal bare-metal **AArch64** OS in Rust. Primary path is QEMU `virt` and PL011 UART ([ADR-003](03-adr/ADR-003-primary-isa-aarch64.md)).

These pages **are** the website. mdBook publishes the same files under `docs/` (see [Docs website + DNS](website.md)). There is no second marketing copy.

This is **not** a product site. Status words need a probe ([honesty ledger](framework/honesty-ledger.md)). Do not say “secure OS,” “production ready,” or “EL0 isolated.” `https://ctos.artof.link` does **not** serve this book until GitHub Pages is live (Route 53 CNAME can already exist).

## What can run today

Dedicated page: [What can run today](overview/what-can-run.md). Three honest samples only:

1. **Cooperative EL1 UART workers** — two heap-stack tasks that print and yield (`sched: task a/b`). Variant: serial heartbeat/counter.
2. **UART RX echo gadget** — PL011 byte in, print out (`input: rx 0x41`). No TTY or line editor.
3. **Standing EL0 stub** — short payload + SVC (`el0: standing` / `restored`). Not a process; no libc or files.

**Cannot run:** Linux binaries, shell, Python, network servers, filesystem apps, SMP.

## Building or porting

Dedicated page: [Building or porting](overview/porting.md).

Today nothing POSIX ports easily (no libc, no dynamic linker, no FS, no public app ABI). The easy path is **in-tree `no_std` Rust** and `cargo build` on `aarch64-ctos.json`. Do not drop in userspace ELFs. A stable SVC ABI is **Planned**.

## Filesystem (Planned)

Dedicated page: [Filesystem: new vs extend](overview/filesystem.md).

**Today:** no VFS, no block stack — nothing compatible out of the box. Do not say “supports FAT.”

**Best fit later:** memfs first, then virtio-blk + FAT16/32 or a tiny xv6-like FS, behind a thin VFS ADR. Avoid ext4/btrfs/ZFS/NTFS as a first cut. Order: VFS ADR → memfs Verified → virtio-blk → on-disk FS → host-checkable image probe.

## KPIs, prerequisites, advantages, drawbacks

| Dedicated page | What it is |
| --- | --- |
| [KPIs / how we measure](overview/measure.md) | CNTPCT, IRQ-delta, boot-delta, ELF size; fail-closed smoke; not SPEC / not a latency budget |
| [Prerequisites](overview/prerequisites.md) | Nightly Rust + QEMU `virt`; Pages not required for kernel work |
| [Advantages](overview/advantages.md) | Document-first, probed claims, QEMU virt scope, pillars as NFRs |
| [Drawbacks / limits](overview/limits.md) | Learning kernel; identity stub at `0x4008_0000`; PAN unclaimed; no net/DMA |

## Deep dives

1. [Product vision](01-vision/product-vision.md)
2. [FR / NFR](02-requirements/fr-nfr.md) — frozen `FR-01`–`FR-15`, `NFR-01`–`NFR-14`
3. [Technical architecture](02-architecture/technical-architecture.md)
4. [Roadmap](04-roadmap/roadmap.md)
5. [Honesty ledger](framework/honesty-ledger.md)
6. [Three pillars](framework/pillars.md)

Kernel build: [GitHub README](https://github.com/artofdream/ctos#readme).

## URLs

| URL | Honesty |
| --- | --- |
| `https://ctos.artof.link` | Route 53 CNAME is **in place**. HTTPS serving these docs is **Planned** until Pages is enabled and a fetch succeeds. |
| `https://artofdream.github.io/ctos/` | Project-site fallback. **Unknown** until a `pages` workflow on `main` is green. |
