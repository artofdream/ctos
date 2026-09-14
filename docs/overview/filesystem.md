# Filesystem: new vs extend

**Today: thin VFS with two backends.** In-RAM **memfs** (A6 / [ADR-027](../03-adr/ADR-027-thin-vfs-memfs.md)) and **FAT16 on virtio-blk** (A7 / [ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). Same `open` / `read` / `write` / `close`. FAT16 write is a depth mile ([ADR-050](../03-adr/ADR-050-fat16-write.md)). FAT16 root listing is a depth mile ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)). Not POSIX `open` / `getdents`, not Linux VFS, not FAT32, not xv6. Track B B5 compares the concepts: [ADR-034](../03-adr/ADR-034-linux-vfs-vs-thin-ctos.md). **Not claiming a Linux filesystem.**

Do not say “supports FAT” as a product. Say the guest read a known FAT16 file when the ledger has `fat: ok`, wrote FAT16 bytes when it has `fat: write` / `fat: create`, and listed root entries when it has `fat: readdir` / `fat: entries`. Hub: [honesty ledger](../framework/honesty-ledger.md), [What can run today](what-can-run.md). Extra stance: [filesystem.md](../framework/filesystem.md).

```mermaid
flowchart TD
  CALL["EL0 / EL1 callers"] --> VFS["Thin VFS<br/>open · read · write · close · readdir"]
  VFS --> MEM["memfs<br/>in-RAM buffers"]
  VFS --> FAT["FAT16<br/>read · write · root readdir"]
  FAT --> BLK["virtio-blk<br/>host fat16.img"]
```

*Same thin API, two backends. Not POSIX `open` / `getdents`. Do not say “supports FAT” as a product.*

## New vs extend

**Prefer implementing a known small filesystem** behind this thin VFS over inventing a novel on-disk format.

| Fit | What | Why |
| --- | --- | --- |
| Landed | **Ramdisk / memfs** | Named heap buffers. Create / lookup / read / write without DMA. |
| Landed | **virtio-blk** + **FAT16** read | Host-visible raw image (`scripts/mkfat16.py`). xv6-like was rejected so the host can inspect the volume. |
| Landed | **FAT16 write** | Guest `vfs::write` on an open FAT handle + backend create of a small file ([ADR-050](../03-adr/ADR-050-fat16-write.md)). |
| Landed | **FAT16 readdir** | Guest `vfs::readdir` lists FAT16 root paths ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)). Not POSIX `getdents`. |
| Later, optional | ctos-specific **virtual mounts** | Prefix / tree mounts. Not a new magic format. A8 is sample **recipes**, not this. |

## Avoid early

Do **not** start with **ext4**, **btrfs**, **ZFS**, or **NTFS**. Those are large, journaled or feature-heavy, and hide the block mile. They are not a first cut.

Host QEMU `-drive` without guest virtio + a VFS read is not a filesystem.

## Roadmap order

One milestone → one branch → one PR.

```mermaid
flowchart LR
  V["1. VFS ADR<br/>ADR-027"] --> M["2. memfs<br/>A6"]
  M --> B["3. virtio-blk<br/>A7"]
  B --> F["4. FAT16 read<br/>A7"]
  F --> H["5. host image + guest read<br/>A7/A9"]
  H --> W["6. FAT16 write<br/>ADR-050"]
  W --> P["7. FAT vs memfs write CNTPCT<br/>ADR-051"]
  P --> R["8. FAT16 readdir<br/>ADR-056"]
```

*A7 is virtio-blk + FAT16 read. ADR-050 is FAT16 write depth. ADR-051 is a same-boot measurement pair (not a bench). ADR-056 is FAT16 root listing. A8 is documented recipes. A9 uses the same volume for `/hello`. Do not say “supports FAT” as a product.*

1. **VFS ADR** — thin interface (create / open / read / write / close of a path).
2. **memfs** — in-RAM named buffers; serial `fs: ok`.
3. **virtio-blk** — virtqueues + sector I/O on QEMU `virt`. Serial `blk: ok`.
4. **On-disk FS** — FAT16 ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). Serial `fat: ok`.
5. **Host-checkable image** — `scripts/mkfat16.py` writes `target/fat16.img`; smoke attaches `-drive if=none,file=…,id=hd0 -device virtio-blk-device,drive=hd0`. Guest `vfs::open("/probe")` must read `fat-hi`. A9 adds `--app` so `/hello` is the published app ELF.
6. **FAT16 write** — guest `vfs::write` on an open FAT handle + backend create of `/fwr` ([ADR-050](../03-adr/ADR-050-fat16-write.md)). Serial `fat: write` / `fat: create`; host `--check-write`.
7. **FAT vs memfs write CNTPCT** — same-boot pair `perf: fat-write` / `perf: memfs-write` / `perf: fs-write-delta` ([ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md)). QEMU TCG lab — not a percent or latency SLA.
8. **FAT16 readdir** — guest `vfs::readdir` lists root thin-VFS paths ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)). Serial `fat: readdir` / `fat: entries`. Not POSIX `getdents`.

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| Thin VFS + memfs create/write/read/close | Serial `fs: ok` + `#[test_case]` | **Verified** (A6; honesty ledger) |
| virtio-blk sector R/W | Serial `blk: ok` + `#[test_case]` | **Verified** on this tip when the ledger has the probe |
| FAT16 `/probe` via the same `open` | Serial `fat: ok` + `#[test_case]` | **Verified** on this tip when the ledger has the probe |
| FAT16 write + small create | Serial `fat: write` / `fat: rewrite` / `fat: create` + host `--check-write` | **Verified** on this tip when the ledger has the probe ([ADR-050](../03-adr/ADR-050-fat16-write.md)) |
| FAT vs memfs write CNTPCT pair | Serial `perf: fat-write` / `perf: memfs-write` / `perf: fs-write-delta` | **Verified** on this tip when the ledger has the probe ([ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md)). Not a bench. |
| FAT16 root readdir | Serial `fat: readdir` / `fat: entries` + `#[test_case]` | **Verified** on this tip when the ledger has the probe ([ADR-056](../03-adr/ADR-056-fat16-readdir.md)). Not POSIX. |
| FAT16 `/hello` app slot (A9) | Serial `slot: ok` + `#[test_case]` | **First cut** when the ledger has the probe |

Do not claim compatibility with anyone’s existing disk.
