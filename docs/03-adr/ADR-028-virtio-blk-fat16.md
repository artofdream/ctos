# ADR-028 — virtio-blk + FAT16 behind the thin VFS

- Status: Accepted (block + FAT mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding app to read a **named file on a disk**, not only an in-RAM buffer. [Issue #38](https://github.com/artofdream/ctos/issues/38) is this mile: QEMU `virt` **virtio-mmio block** (virtqueues, DMA, sector R/W) plus one on-disk FS, mounted through the A6 thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md)).

Host QEMU `-drive` without guest code is **not** a probe. memfs stays one backend. This mile must not invent a second `open` story. Not POSIX. Not ext4. Not app hosting. A8–A9 stay Planned.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | virtio-mmio blk (poll used ring) + **FAT16** on a host-built raw image. Same `VfsOps` / `vfs::open` as memfs. Known file `/probe` holds `fat-hi`. | Host can inspect the image (`scripts/mkfat16.py --check`). Guest DMA + VFS read is the probe. |
| **H2 (rejected)** | Tiny xv6-like inode FS. | Fine as a teaching layout, but the host cannot `file` / mount / check the image without a ctos-specific tool. Issue #38 prefers a host-visible image. |
| **H3 (rejected)** | FAT32. | Root is a cluster chain; more code for the same probe. A 4 MiB image is naturally FAT16 once cluster count ≥ 4085. |
| **H4 (rejected)** | A second `open` / `mount` SVC, or a `/d/` prefix that breaks the A6 path grammar. | ADR-027 forbids a second open story. Paths stay `/` + `[a-z0-9_-]`. |
| **H5 (rejected)** | virtio-blk-pci, virtio IRQ via GIC, or write-back FAT this mile. | PCI and IRQs are extra surface. FAT is **read-only** (guest `write` → `FsError::ReadOnly`). Sector R/W is proven on an unused last sector, not by mutating the volume. |

## Decision

1. **On-disk format is FAT16.** Chosen because the smoke host can build and inspect a raw image without a ctos-specific decoder. xv6-like and FAT32 stay unchosen. Do not say “supports FAT” as a product; say “the guest read a known FAT16 file from a virtio-blk image.”
2. **virtio-mmio on QEMU `virt`.** Scan transports at `0x0a000000 + n×0x200` (32 slots) for Device ID 2. QEMU 8.2 `virt` ships **legacy version 1** on those transports (page-aligned `QueuePFN`); modern version 2 is also accepted. One split virtqueue. DMA addresses are **identity PAs** of a 4 KiB-aligned `.bss` buffer (`.data`/heap stay identity-mapped; [ADR-025](ADR-025-identity-rodata-tear.md)). Poll `used.idx` after `QueueNotify`. Cache maintenance (`dc civac`) covers the **whole** DMA object (desc/avail and used/req/status/data) — a page-0-only sync leaves a stale request on write-back D-cache. No virtio IRQ this mile. Device MMIO already sits in the Device-nGnRnE L1 block.
3. **Thin VFS, two backends.** `vfs::open` / `read` / `write` / `close` stay the only story. memfs is tried first. If the name is missing there, FAT16 root (8.3, no extension → `/probe` for `PROBE`) is tried. `create` stays memfs and fails with `Exists` if FAT already has the name. FAT `write` / `create` return `ReadOnly`. Same path grammar as A6. No extra `/`. No new SVC numbers.
4. **Host image.** `scripts/mkfat16.py` writes a 4 MiB raw FAT16 volume (cluster count ≥ 4085 so the type is FAT16, not FAT12) with one file `PROBE` / contents `fat-hi`. `scripts/qemu-smoke.sh`, `scripts/qemu-aarch64.sh`, and `scripts/qemu-serial-inject.py` attach it as:
   `-drive if=none,file=<img>,format=raw,id=hd0 -device virtio-blk-device,drive=hd0`
5. **Fail-closed probe.** virtio capacity + last-sector write/read prints `blk: virtio` / `blk: cap` / `blk: rw` / `blk: ok`. VFS open/read of `/probe` prints `fat: mount` / `fat: read` / `fat: ok`. `scripts/qemu-smoke.sh` greps those and rejects `blk: probe missed` / `fat: probe missed`. Host `--check` of the image is extra, not a substitute for the guest. `#[test_case]` covers sector R/W, VFS `/probe`, and a missing FAT name. Existing `svc:*` / `libctos:*` / `loader:*` / `el0: task-*` / `ident:*` / `pan:*` / `fs:*` markers stay.
6. **Honesty.** Say “the guest programmed virtio-blk and read a known FAT16 file through the thin VFS” only when the serial / tests pass. First hello on a modern-only (`Version==2`) scan was `blk: probe missed` — QEMU 8.2 `virt` exposes legacy version 1. After accepting v1 + `QueuePFN`, this cloud VM printed `blk: ok` and `fat: ok`. Do **not** say: app hosting is done, POSIX, Linux VFS, FAT32, xv6, writeable volumes, virtio-net, “EL0 isolated,” or “secure OS.” Track A A8–A9 stay Planned. Host `-drive` without guest virtio + VFS read is not a probe.
7. **NFR-10 text** is revised in place (ID unchanged) to name this virtio-blk + FAT16 mile. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/virtio.rs`, `src/fat.rs`, `src/vfs.rs` (dispatch + `ReadOnly`), `scripts/mkfat16.py`, QEMU `-drive` in the smoke / runner scripts.
- Docs: [filesystem.md](../framework/filesystem.md), [overview/filesystem.md](../overview/filesystem.md), [track-a.md](../04-roadmap/track-a.md), threat-model v1.15. Roadmap cites #38 / Track A #31.
- A8 (documented sample apps, [issue #39](https://github.com/artofdream/ctos/issues/39)) is not this PR. That mile should teach an in-tree payload that uses the existing SVC + VFS (`fs_open` / `fs_read` of `/probe` or a memfs name) — not a new ABI and not a second FS.
