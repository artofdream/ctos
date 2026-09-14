# ADR-050 — FAT16 write behind the thin VFS

- Status: Accepted (FAT16 write mile Verified when the serial / tests / host-check pass; Track A / app hosting still Planned)
- Date: 2026-09-13

## Context

[ADR-028](ADR-028-virtio-blk-fat16.md) landed virtio-blk + **read-only** FAT16 behind the thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md)). Guest `write` on a FAT handle returned `ReadOnly`. Sector R/W was proven on an unused last sector, not by mutating the volume.

Docs “new vs extend” prefer deepening the **known small FS** (FAT16) over inventing a format or jumping to ext4 / FAT32 / xv6. This mile is Future slice 3: guest FAT16 **write** (and a small create) on the virtio-blk volume, same thin VFS, fail-closed serial + tests + optional host check.

Not POSIX `open`/`write`. Not FAT32. Not “supports FAT” as a product. Not app hosting. A9 `/hello` load path must keep working.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Guest **FAT16 write** on an open handle via the same `vfs::write`, plus FAT backend **create** of a small single-cluster file. Serial `fat: write` / `fat: rewrite` / `fat: create` / keep `fat: ok`. Host `--check-write` sees `/fwr`. | Deepens ADR-028. Same path grammar. Caps: one cluster, `FILE_MAX` (256). |
| **H2 (alt, rejected this PR)** | Directory listing / `readdir`-style thin VFS mile only. | Write is in scope and fits one PR. Listing is a later depth mile. |
| **H3 (rejected)** | FAT32, ext4, xv6 FS, or a Linux VFS clone. | Forbidden by overview + ADR-028. |
| **H4 (rejected)** | Route `vfs::create` to FAT when mounted. | Would put A6 `/kprobe` / `/eprobe` on the volume; `vfs::reset` cannot clear them; second create → `Exists`. `vfs::create` stays **memfs-first**. FAT create is `fat::create` (backend), then confirmed with `vfs::open` / `read`. |
| **H5 (rejected)** | Claim POSIX `write` or “writeable FAT volumes” as a product. | Honesty: say the guest wrote FAT16 bytes when markers pass. |

## Decision

1. **FAT `write` on an open handle.** Updates the file’s first (only) cluster and the root dirent size. Offset starts at 0 on `open`. Empty write → `TooBig` (same shape as memfs). Refuse multi-cluster grow and sizes above `FILE_MAX` / one cluster this mile.
2. **Probe rewrite of `/probe`.** `vfs::open("/probe")` → write `fat-wr` → read back → write `fat-hi` restore → `fat: write` / `fat: rewrite`. Keeps later `/probe` readers and tests stable.
3. **FAT backend create of `/fwr`.** `fat::create("/fwr")` allocates one free cluster + root dirent, writes `fat-nw`, then `vfs::open` / `read` confirms. Serial `fat: create`. Left on the image for host `--check-write`.
4. **`vfs::create` stays memfs-first** (A6). Do not invent a second open story; do not put memfs probe names on the volume.
5. **Fail-closed smoke.** Grep `fat: write` / `fat: rewrite` / `fat: create` / `fat: ok`. Reject `fat: probe missed`. Keep A9 `slot: ok` / cross-update. Host `scripts/mkfat16.py --check-write` requires `/fwr=fat-nw` and restored `/probe=fat-hi`.
6. **Honesty.** Say “the guest wrote FAT16 bytes through the thin VFS (and created a small FAT16 file)” only when serial / tests / host-check pass. Do **not** say: POSIX, FAT32, “supports FAT,” app hosting done, EL0 isolated, PAN, taken SError.
7. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.28**. Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (write / create / restore probe), `scripts/mkfat16.py --check-write`, `scripts/qemu-smoke.sh` greps.
- Docs: this ADR, [filesystem.md](../overview/filesystem.md), [framework/filesystem.md](../framework/filesystem.md), honesty ledger, roadmap note, threat-model v1.28.
- Follow-ups (not this PR): multi-cluster grow, dirent shrink helpers, `vfs::create` → FAT without breaking A6, delete/unlink. Root `readdir`: [ADR-056](ADR-056-fat16-readdir.md).
