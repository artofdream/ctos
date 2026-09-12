# Filesystem stance

**In-RAM memfs exists; no on-disk filesystem.** A thin VFS (`src/vfs.rs`, [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)) serves named heap buffers. There is no directory tree, no block device, and no on-disk format. A `grep` of `src/` finds no virtio-blk, FAT, or inode code. That absence is the probe for “no disk.”

Do not say “ctos has files” as if a volume existed. Do not mint a new FR/NFR ID in chat. Frozen Out list already names POSIX / userspace as later ([fr-nfr.md](../02-requirements/fr-nfr.md)). Hub: [overview.md](overview.md). Samples that do run: [apps-today.md](apps-today.md).

Site source of truth: [Filesystem](../overview/filesystem.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## Order

Each later step is one milestone → one branch → one PR, with its own ADR when it lands.

| Step | What it is | Why this order | Status |
| --- | --- | --- | --- |
| 1. **memfs** | In-RAM named buffers on the existing first-fit heap (`src/heap.rs`). Create / lookup / read / write of a path without DMA. | Same “easiest in-tree” shape as a coop EL1 task. No virtqueue, no disk image. | **Verified** (A6 / ADR-027). Serial `fs: ok`. |
| 2. **virtio-blk** | QEMU `virt` virtio block: virtqueues, a guest-visible disk, read/write sectors. | Paging and a heap already exist, but virtio-mmio / DMA is its own mile. Do not pretend PL011 RX is a block device. | **Planned** (A7). |
| 3. **FAT or xv6-like** | An on-disk layout on top of the block device. FAT if we want a host-visible image; xv6-like if we want a tiny teaching inode FS. | Choose in the ADR that lands it. This page does **not** pick. | **Planned** (A7). |

`fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` are fail-closed in `scripts/qemu-smoke.sh`. Invent `blk: ok` with the A7 PR.

## What this is not

- Not POSIX `open` / `stat` / `mount`. Handles are a thin VFS fd, not a Linux fd table ([building-or-porting.md](building-or-porting.md)).
- Not virtio-net, 9p, or a Linux rootfs.
- Not “we have a disk because QEMU can attach one.” Host `-drive` without guest code is not a probe.
- Not a claim that Python, a shell, or a package manager becomes possible once memfs exists. Those still need more than a RAM tree. App hosting stays **Planned**.

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| memfs create/write/read/close | Serial `fs: ok` + `#[test_case]` | **Verified** on this tip (honesty ledger) |
| No virtio-blk / FAT / inode in this tree | Source: no virtio-blk / FAT in `src/` | Verified (absence) |
| virtio-blk works | QEMU disk + guest driver + marker | **Planned** |
| FAT or xv6-like on a block device | Format + read-back probe | **Planned** |

Unprobed stays **Unknown**. File presence of this note is not an on-disk filesystem. See the [honesty ledger](honesty-ledger.md).
