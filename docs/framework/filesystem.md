# Filesystem stance

**No filesystem today.** There is no VFS, no `open`/`read`/`write` of named files, no directory, no block device, and no on-disk format. A `grep` of `src/` finds no memfs, virtio-blk, FAT, or inode code. That absence is the probe for “none now.” The capability itself is **Planned**.

Do not say “ctos has files.” Do not mint a new FR/NFR ID in chat. Frozen Out list already names POSIX / userspace as later ([fr-nfr.md](../02-requirements/fr-nfr.md)). Hub: [overview.md](overview.md). Samples that do run: [apps-today.md](apps-today.md).

A docs website at https://ctos.artof.link is **Planned**. This tree does not publish Pages.

## Intended order (not started)

Each step is one milestone → one branch → one PR, with its own ADR when it lands. Later steps do not start on an unclosed earlier one.

| Step | What it would be | Why this order | Status |
| --- | --- | --- | --- |
| 1. **memfs** | In-RAM named buffers on the existing first-fit heap (`src/heap.rs`). Prove create / lookup / read / write of a path without DMA. | Same “easiest in-tree” shape as a coop EL1 task. No virtqueue, no disk image. | **Planned.** Not in tree. |
| 2. **virtio-blk** | QEMU `virt` virtio block: virtqueues, a guest-visible disk, read/write sectors. | Paging and a heap already exist, but virtio-mmio / DMA is its own mile. Do not pretend PL011 RX is a block device. | **Planned.** After memfs (or justified in that ADR if a probe needs a disk first). |
| 3. **FAT or xv6-like** | An on-disk layout on top of the block device. FAT if we want a host-visible image; xv6-like if we want a tiny teaching inode FS. | Choose in the ADR that lands it. This page does **not** pick. | **Planned.** After a block device. |

A smoke string such as `fs: ok` / `blk: ok` would be invented **with** that PR, fail-closed in `scripts/qemu-smoke.sh`. Until then, there is nothing to grep.

## What this is not

- Not POSIX `open` / `stat` / `mount`. Those wait on a later SVC ABI ([building-or-porting.md](building-or-porting.md)).
- Not virtio-net, 9p, or a Linux rootfs.
- Not “we have a disk because QEMU can attach one.” Host `-drive` without guest code is not a probe.
- Not a claim that Python, a shell, or a package manager becomes possible once memfs exists. Those still need an ABI, a process, and usually more than a RAM tree.

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| No filesystem in this tree | Source: no VFS / memfs / virtio-blk / FAT in `src/` | Verified (absence) |
| Guest can open a file | Serial + `#[test_case]` that do not exist yet | **Planned** |
| virtio-blk works | QEMU disk + guest driver + marker | **Planned** |
| FAT or xv6-like on a block device | Format + read-back probe | **Planned** |

Unprobed stays **Unknown**. File presence of this note is not a filesystem. See the [honesty ledger](honesty-ledger.md).
