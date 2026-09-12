# Filesystem stance

**Thin VFS with two backends.** memfs (A6 / [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)) is in-RAM named buffers. FAT16 on virtio-blk (A7 / [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)) is a read-only on-disk volume. Same `vfs::open` / `read` / `write` / `close`. No directory tree, no POSIX `mount`, no FAT32, no xv6-like inode FS. Linux VFS objects vs this thin surface: [ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md) (Track B B5). **Not claiming a Linux filesystem.**

Do not say “ctos has files” as if it were a Linux volume. Do not mint a new FR/NFR ID in chat. Frozen Out list already names POSIX / userspace as later ([fr-nfr.md](../02-requirements/fr-nfr.md)). Hub: [overview.md](overview.md). Samples that do run: [apps-today.md](apps-today.md).

Site source of truth: [Filesystem](../overview/filesystem.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## Order

Each step is one milestone → one branch → one PR.

| Step | What it is | Why this order | Status |
| --- | --- | --- | --- |
| 1. **memfs** | In-RAM named buffers on the first-fit heap (`src/heap.rs`). Create / lookup / read / write of a path without DMA. | Same “easiest in-tree” shape as a coop EL1 task. | **Verified** (A6 / ADR-027). Serial `fs: ok`. |
| 2. **virtio-blk** | QEMU `virt` virtio-mmio block: virtqueues, DMA, sector R/W. | Host `-drive` is not a probe. | **Verified** when the ledger has `blk: ok` (A7 / ADR-028). |
| 3. **FAT16** | Host-visible 4 MiB raw image; guest opens `/probe` (`PROBE`) and reads `fat-hi`. A9 also stores the app ELF as `/hello`. | Chosen over xv6-like so the host can inspect the image (`scripts/mkfat16.py --check`). FAT32 / xv6 rejected this mile. | **Verified** when the ledger has `fat: ok`. Read-only. `/hello` is the A9 slot, not a second FS. |

`fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` stay fail-closed. A7 adds `blk: virtio` / `blk: cap` / `blk: rw` / `blk: ok` and `fat: mount` / `fat: read` / `fat: ok`.

QEMU flags (smoke + cargo runner):

`-drive if=none,file=target/fat16.img,format=raw,id=hd0 -device virtio-blk-device,drive=hd0`

## What this is not

- Not POSIX `open` / `stat` / `mount`. Handles are a thin VFS fd, not a Linux fd table ([building-or-porting.md](building-or-porting.md)).
- Not virtio-net, 9p, or a Linux rootfs.
- Not “we have a disk because QEMU can attach one.” Host `-drive` without guest virtio + a VFS read is not a probe.
- Not writeable FAT. Guest `write` on a FAT handle is `ReadOnly`.
- Not app hosting. A8 is documented sample **rebuild recipes** ([what-can-run.md](../overview/what-can-run.md)).

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| memfs create/write/read/close | Serial `fs: ok` + `#[test_case]` | **Verified** (A6; honesty ledger) |
| virtio-blk works | Serial `blk: ok` + sector R/W `#[test_case]` | **Verified** on this tip when the ledger has the probe |
| FAT16 on that device | Serial `fat: ok`; VFS `/probe` = `fat-hi` | **Verified** on this tip when the ledger has the probe |

Unprobed stays **Unknown**. File presence is not that probe. See the [honesty ledger](honesty-ledger.md).
