# Daily brief — 2026-09-12 (A7 virtio-blk + FAT16)

## Where we stopped

PR #57 (`cursor/a7-virtio-blk-fat-a587`) — Track A / A7 / [ADR-028](../../docs/03-adr/ADR-028-virtio-blk-fat16.md). Parent [#31](https://github.com/artofdream/ctos/issues/31), child [#38](https://github.com/artofdream/ctos/issues/38).

Chose **FAT16** (host-visible 4 MiB image) over xv6-like / FAT32. Same thin VFS `open` as memfs. FAT is read-only.

Cloud `qemu-smoke` **Verified** on `b4903e8`: `blk: virtio` / `blk: cap sectors=8192` / `blk: rw` / `blk: ok` / `fat: mount` / `fat: read` / `fat: ok`. 79 tests + force-fail. QEMU `virt` virtio-mmio is **legacy v1** (first hello attempt Failed: modern-only scan). GHA on that SHA also Verified (PR [34687722316](https://github.com/artofdream/ctos/actions/runs/34687722316) both matrices). Host `-drive` is not the probe.

Bugbot Medium: `cache_sync` only covered page 0. Follow-up syncs the whole `Dma` (desc/avail + used/req/status/data).

Author does not merge (ADR-002). A8–A9 stay Planned.

## Do next

1. MRC on #57 (new session). Owner merges if `cursor[bot]` authored.
2. **A8** ([#39](https://github.com/artofdream/ctos/issues/39)): documented sample apps — in-tree payload using existing SVC + VFS (`fs_open` / `fs_read` of `/probe` or a memfs name). Not a new ABI.

## Honesty

Guest virtio + VFS `/probe` = `fat-hi` is the probe. File presence is not that. Do not say “supports FAT” as a product. App hosting stays Planned.
