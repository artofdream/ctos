# ADR-107 — Track A — FAT32 read path beyond FAT16 (second virtio-blk)

- Status: Accepted (runtime **Verified** only when this tip's `qemu-smoke` prints `fat32: ok` and the broken-chain negative prints `fat32: fail reason=bad-chain`; otherwise Documented)
- Date: 2026-10-23

## Context

[ADR-028](ADR-028-virtio-blk-fat16.md) put FAT16 on virtio-blk behind the thin VFS, and later ADRs grew write, readdir, unlink, grow, mkdir and rmdir on it. `fat.rs` rejects any volume with 65525 or more clusters, so a FAT32 volume (the kind `mkfs.fat` makes for anything past a few hundred MiB) cannot be read at all. The sponsor asked for FAT32 support on the existing virtio-blk path, tested in QEMU against an image built by `mkfs.fat -F 32`, with a fail-closed probe and a negative.

FAT32 differs from FAT16 in three places that matter here:

1. **BPB.** `BPB_RootEntCnt` and `BPB_FATSz16` are 0; the FAT size is the 32-bit `BPB_FATSz32`, and the root directory's first cluster is `BPB_RootClus`. `BPB_FSInfo` names the FSInfo sector, which carries three signatures and a free-cluster hint.
2. **FAT table.** Entries are 32 bits wide, of which the low 28 are used. End-of-chain is `>= 0x0FFFFFF8`; `0x0FFFFFF7` is a bad cluster.
3. **Root directory.** There is no fixed root region. The root is an ordinary cluster chain.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | A separate read-only FAT32 reader (`src/fat32.rs`) on a **second** virtio-blk device holding a `mkfs.fat -F 32` image. The FAT16 disk, `fat.rs` and the VFS stay unchanged. | Smallest honest step. FAT16 regressions are impossible by construction, and the FAT16 smoke still runs on the same boot. |
| **H2 (rejected)** | Widen `fat.rs` (`u16` clusters everywhere) to FAT32 with write support in one PR. | Touches every FAT16 write path at once. Too large to review and test honestly in one step. |
| **H3 (rejected)** | Append the FAT32 volume to the FAT16 image at a fixed LBA. | The FAT16 host checks (`mkfat16.py --check`) pin the image size, and a magic offset is a hidden contract. |
| **H4 (rejected)** | Hand-roll the FAT32 image in Python. | The sponsor asked for `mkfs.fat -F 32`. A real tool also catches reader assumptions a home-made writer would share. |

## Decision

1. **Disk.** QEMU gets a second `-drive …,readonly=on` + `-device virtio-blk-device` with `target/fat32.img`, listed **before** the FAT16 disk. QEMU fills virtio-mmio slots from the top, so the FAT16 disk stays the guest's first-found blk device, and older kernels (the cross-update prior OS) keep using it. `virtio::init2()` programs the second transport in scan order with its own queue memory; it exposes `read_sector2` only (no write).
2. **Image.** `scripts/mkfat32.sh` runs `mkfs.fat -F 32 -s 1 -S 512` on 34 MiB (about 68k clusters, above the 65525 floor), then uses `mcopy` to add:
   - `PROBE32.TXT` (`fat32-hi\n`);
   - `BIG.BIN` (3000 deterministic bytes, 6 clusters);
   - 20 small files, so the root directory needs more than one cluster;
   - `LAST.TXT` (`fat32-root-chain\n`), which therefore lands in a later root cluster.

   `mtools` allocates forward, so the script then moves `BIG.BIN`'s last three clusters to a free run near cluster 40000 and relinks them in every FAT copy. `fsck.fat -n` must pass on the result (a host check, not a guest probe).
3. **Reader (`fat32.rs`).** It does the following:
   - Mount checks the 0x55AA signature, `bps == 512`, a power-of-two `spc`, and `RootEntCnt == 0` and `FATSz16 == 0` (otherwise `NotFat32`). It requires a cluster count of at least 65525 (otherwise `NotFat32`), checks that the FAT fits every cluster index and that the root cluster is in range, and checks the FSInfo lead/struct/trail signatures plus a free-count bound.
   - Chain walk masks entries to 28 bits and ends at `>= 0x0FFFFFF8`. A free (0), reserved (1), bad (`0x0FFFFFF7`) or out-of-range link is `BadLink`. The walk is bounded by the cluster count, so a cycle is `Cycle`.
   - File read requires the chain length to equal `ceil(size / cluster)` exactly (otherwise `SizeMismatch`).
   - Directory scan walks the root chain, stops at the first `0x00` entry, and skips deleted, LFN and volume-label entries. It matches 8.3 names only.
4. **Probe (boot path, after the FAT16 probe).** It prints:
   - `fat32: mount clusters=N root=R fsinfo=ok`
   - `fat32: read /probe32.txt bytes=9`
   - `fat32: chain /big.bin clusters=6 jumps=J bytes=3000`, where J ≥ 1 non-adjacent links and the content matches byte for byte
   - `fat32: root-chain clusters=K last=found` (K ≥ 2)
   - `fat32: ok`

   Any error prints `fat32: fail reason=<reason>` and the kernel prints `fat32: probe missed`.
5. **Smoke (fail closed).** `qemu-smoke.sh` requires every marker above, plus `clusters >= 65525`, `jumps >= 1` and root clusters `>= 2`, and rejects any `fat32: fail` or `probe missed`. **Negative:** a second boot of the same kernel with a copy of the image whose `FAT[root]` is set to free (`mkfat32.sh --corrupt`) must print `fat32: fail reason=bad-chain`, must not print `fat32: ok`, and must still print FAT16 `fat: ok`.
6. **`cargo test` (in QEMU).** The tests are:
   - in-memory FAT32 images covering a good read through a two-cluster root, a free link, a cycle, bad and out-of-range links, a chain longer than the size, bad FSInfo, a FAT16-sized cluster count and a non-zero fixed root;
   - the real-disk probe;
   - a check that the FAT16 primary disk is rejected by the FAT32 mount (`NotFat32`).
7. **CI.** `dosfstools` and `mtools` are added to the smoke jobs' apt lines and to the Dockerfile.

## Not claimed

- No FAT32 write, create, delete or grow. No VFS mount of the FAT32 volume. No LFN decoding. No subdirectory walk on FAT32 (only the root chain). No FSInfo update. No FAT mirror comparison (copy 0 is read). No exFAT.
- Not a performance claim. Not a real-hardware claim (QEMU `virt` only).
- FAT16 behaviour is unchanged. The FAT16 VFS is still the only mounted FAT.

## Consequences

- QEMU runs (smoke, `cargo test`, cross-update) now need `mkfs.fat`, `mcopy` and `fsck.fat` on the host. `mkfat32.sh` fails loudly if they are missing.
- A later ADR can mount FAT32 behind the VFS or add write support, starting from this reader.
