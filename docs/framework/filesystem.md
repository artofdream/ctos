# Filesystem stance

**Thin VFS with two backends and prefix mounts.** memfs (A6 / [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)) is in-RAM named buffers. FAT16 on virtio-blk (A7 / [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)) is an on-disk volume; write depth is [ADR-050](../03-adr/ADR-050-fat16-write.md); multi-cluster grow is [ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md); root listing is [ADR-056](../03-adr/ADR-056-fat16-readdir.md); root delete is [ADR-057](../03-adr/ADR-057-fat16-delete.md); root mkdir is [ADR-073](../03-adr/ADR-073-fat16-mkdir.md); nested mkdir + empty rmdir is [ADR-077](../03-adr/ADR-077-fat16-nested-rmdir.md); EL0 `fs_mkdir` is [ADR-075](../03-adr/ADR-075-el0-fs-mkdir.md). Path prefixes route through a mount table ([ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md)). Same `vfs::open` / `read` / `write` / `close` (+ `readdir` / `unlink` / `mkdir` / `rmdir` for FAT). No POSIX `mount` / `getdents` / `unlink` / `rmdir`, no FAT32, no xv6-like inode FS. Linux VFS objects vs this thin surface: [ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md) (Track B B5). **Not claiming a Linux filesystem.**

Do not say “ctos has files” as if it were a Linux volume. Do not mint a new FR/NFR ID in chat. Frozen Out list already names POSIX / userspace as later ([fr-nfr.md](../02-requirements/fr-nfr.md)). Hub: [overview.md](overview.md). Samples that do run: [apps-today.md](apps-today.md).

Site source of truth: [Filesystem](../overview/filesystem.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## Order

Each step is one milestone → one branch → one PR.

| Step | What it is | Why this order | Status |
| --- | --- | --- | --- |
| 1. **memfs** | In-RAM named buffers on the first-fit heap (`src/heap.rs`). Create / lookup / read / write of a path without DMA. | Same “easiest in-tree” shape as a coop EL1 task. | **Verified** (A6 / ADR-027). Serial `fs: ok`. |
| 2. **virtio-blk** | QEMU `virt` virtio-mmio block: virtqueues, DMA, sector R/W. | Host `-drive` is not a probe. | **Verified** when the ledger has `blk: ok` (A7 / ADR-028). |
| 3. **FAT16** | Host-visible 4 MiB raw image; guest opens `/probe` (`PROBE`) and reads `fat-hi`. Same volume also stores freestanding samples `/hello` (A9), `/fsdemo` (ADR-059), `/fatdemo` (ADR-061), `/yldemo` (ADR-062), `/netdemo` (ADR-068), `/udpdemo` (ADR-071), `/mkdemo` (ADR-075). | Chosen over xv6-like so the host can inspect the image (`scripts/mkfat16.py --check`). FAT32 / xv6 rejected this mile. | **Verified** when the ledger has `fat: ok`. Write depth: [ADR-050](../03-adr/ADR-050-fat16-write.md). Multi-cluster grow: [ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md). Root listing: [ADR-056](../03-adr/ADR-056-fat16-readdir.md). Root delete: [ADR-057](../03-adr/ADR-057-fat16-delete.md). Sample paths are catalog slots, not a second FS. |
| 4. **Prefix mounts** | Mount table routes `/mem` (+ A6 names) → memfs and `/` → FAT16. | Product-depth virtual mounts without a new disk format. | **Verified** when the ledger has `vfs: mounts` ([ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md)). Not `mount(2)`. |

`fs: create` / `fs: write` / `fs: read` / `fs: el0` / `fs: ok` stay fail-closed. A7 adds `blk: virtio` / `blk: cap` / `blk: rw` / `blk: ok` and `fat: mount` / `fat: read` / `fat: ok`. ADR-050 adds `fat: write` / `fat: rewrite` / `fat: create`. ADR-064 adds `fat: grow`. ADR-056 adds `fat: readdir` / `fat: entries`. ADR-057 adds `fat: delete`. ADR-073 adds `fat: mkdir`. ADR-077 adds `fat: nested` / `fat: rmdir`. ADR-075 adds EL0 `fs_mkdir` / `libctos: mkdir-ok` / `mkdemo: ok`. ADR-058 adds `vfs: mount` / `vfs: mounts`.

QEMU flags (smoke + cargo runner):

`-drive if=none,file=target/fat16.img,format=raw,id=hd0 -device virtio-blk-device,drive=hd0`

## What this is not

- Not POSIX `open` / `stat` / `mount`. Handles are a thin VFS fd, not a Linux fd table ([building-or-porting.md](building-or-porting.md)).
- Not virtio-net, 9p, or a Linux rootfs.
- Not “we have a disk because QEMU can attach one.” Host `-drive` without guest virtio + a VFS read is not a probe.
- Not POSIX writeable volumes. Guest FAT write is a thin-VFS depth mile ([ADR-050](../03-adr/ADR-050-fat16-write.md)); multi-cluster grow is [ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md); say “wrote / grew FAT16 bytes” when markers pass. Not a POSIX write API.
- Not POSIX `getdents` / `opendir`. Guest FAT root listing is a thin-VFS depth mile ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)); say “listed FAT16 root entries” when markers pass.
- Not POSIX `unlink` / `remove`. Guest FAT root delete is a thin-VFS depth mile ([ADR-057](../03-adr/ADR-057-fat16-delete.md)); say “deleted a FAT16 directory entry” when markers pass.
- Not POSIX `mount(2)` / Linux vfsmount. Prefix mounts are a thin-VFS depth mile ([ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md)); say “routed path prefixes through a mount table” when markers pass.
- Not app hosting. A8 is documented sample **rebuild recipes** ([what-can-run.md](../overview/what-can-run.md)).

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| memfs create/write/read/close | Serial `fs: ok` + `#[test_case]` | **Verified** (A6; honesty ledger) |
| virtio-blk works | Serial `blk: ok` + sector R/W `#[test_case]` | **Verified** on this tip when the ledger has the probe |
| FAT16 on that device | Serial `fat: ok`; VFS `/probe` = `fat-hi` | **Verified** on this tip when the ledger has the probe |
| FAT16 write + small create | Serial `fat: write` / `fat: create`; host `--check-write` | **Verified** on this tip when the ledger has the probe ([ADR-050](../03-adr/ADR-050-fat16-write.md)) |
| FAT16 multi-cluster grow | Serial `fat: grow` / `libctos: fat-grow` | **Verified** on this tip when the ledger has the probe ([ADR-064](../03-adr/ADR-064-fat16-multi-cluster-grow.md)) |
| FAT16 root mkdir | Serial `fat: mkdir` | **Verified** on this tip when the ledger has the probe ([ADR-073](../03-adr/ADR-073-fat16-mkdir.md)) |
| EL0 `fs_mkdir` | Serial `libctos: mkdir-ok` / `mkdemo: ok` | **Verified** on this tip when the ledger has the probe ([ADR-075](../03-adr/ADR-075-el0-fs-mkdir.md)) |
| FAT16 nested mkdir + empty rmdir | Serial `fat: nested` / `fat: rmdir` | **Verified** on this tip when the ledger has the probe ([ADR-077](../03-adr/ADR-077-fat16-nested-rmdir.md)) |
| FAT16 root readdir | Serial `fat: readdir` / `fat: entries` | **Verified** on this tip when the ledger has the probe ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)) |
| FAT16 root delete | Serial `fat: delete` | **Verified** on this tip when the ledger has the probe ([ADR-057](../03-adr/ADR-057-fat16-delete.md)) |
| Thin VFS prefix mounts | Serial `vfs: mount` / `vfs: mounts` | **Verified** on this tip when the ledger has the probe ([ADR-058](../03-adr/ADR-058-vfs-prefix-mounts.md)) |

Unprobed stays **Unknown**. File presence is not that probe. See the [honesty ledger](honesty-ledger.md).
