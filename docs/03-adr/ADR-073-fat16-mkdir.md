# ADR-073 — FAT16 mkdir (directory create depth)

- Status: Accepted (FAT16 root mkdir mile Verified when the serial / tests pass; Track A / app hosting still Planned for Linux/POSIX claims; Track N untouched)
- Date: 2026-09-16

## Context

[ADR-028](ADR-028-virtio-blk-fat16.md) landed virtio-blk + FAT16 **read**. [ADR-050](ADR-050-fat16-write.md) deepened **write** / small create. [ADR-056](ADR-056-fat16-readdir.md) deepened **root listing**. [ADR-057](ADR-057-fat16-delete.md) deepened **root file delete**. [ADR-064](ADR-064-fat16-multi-cluster-grow.md) deepened **multi-cluster file grow**. Docs “new vs extend” prefer deepening the known small FS (FAT16) behind the thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md)).

Sponsor train mile (2) is honest FAT16 **directory create** (`mkdir`): allocate a root dirent with `ATTR_DIR`, allocate an empty directory cluster, write `.` / `..`, update the parent (root). Fail-closed serial + `#[test_case]`.

Not POSIX `mkdir` / `mkdirat`. Not a nested path tree (thin-VFS path grammar stays flat `/name`). Not exFAT / FAT32. Not recursive `rm`. Do **not** implement thin TCP (sponsor mile 3) in this PR.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Kernel + thin VFS `vfs::mkdir` creates a FAT16 **root** directory. Serial `fat: mkdir` / keep `fat: ok`. Probe creates `/fdir`, confirms dirent + `.`/`..` cluster, refuses second create (`Exists`). No new SVC this mile. | Deepens ADR-028/050/056/057/064. Same flat path grammar. |
| **H2 (rejected)** | Claim POSIX `mkdir` / `mkdirat` or a Linux mkdir ABI. | Forbidden honesty. |
| **H3 (rejected)** | New `SVC` / `libctos` wrapper this PR. | Delete/readdir/grow miles used existing VFS; mkdir is a kernel/VFS depth probe; EL0 SVC can follow later. |
| **H4 (rejected)** | Nested subdirectory trees, LFN, recursive `rm`, or exFAT. | Out of scope; follow-ups. |
| **H5 (rejected)** | Thin TCP / Track N mile 3 in this PR. | Separate sponsor mile; out of scope. |

## Decision

1. **`fat::mkdir` / `vfs::mkdir`.** Allocate a free cluster (EOC), zero it, write `.` (self cluster) and `..` (parent = FAT16 root → cluster 0), allocate a free root dirent with `ATTR_DIR` + size 0. Name collision with a file **or** directory → `FsError::Exists` (fail-closed). Missing mount / bad path → fail-closed.
2. **Probe after grow.** Create `/fdir` (leftover empty dir: `rmdir_empty` then recreate). Confirm `has_dir`, refuse `open` as a file (`Missing`), confirm `.`/`..` bytes, second `mkdir` → `Exists`. Serial `fat: mkdir`. Keep `fat: ok` / write / readdir / delete / grow / A9 / net markers.
3. **No new SVC numbers.** Public ABI stays as today (FS 19–23, net 24–26). Not a POSIX product claim. EL0/`libctos` mkdir is an explicit follow-up.
4. **Fail-closed smoke.** Grep `fat: mkdir` / keep `fat: ok` / grow / delete / net / samples. `#[test_case]` covers mkdir + exists/bad-path + create-on-dir-name refuse.
5. **Honesty.** Say “the guest created a FAT16 root directory through the thin VFS” only when serial / tests pass. Do **not** say: POSIX `mkdir`, directory trees, exFAT/FAT32, “supports FAT,” EL0 isolated, PAN, taken SError, Linux, containers, TCP.
6. **NFR-10 text** revised in place (ID unchanged). Threat-model **v1.48**. Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (mkdir / dot entries / probe), `src/vfs.rs` (`vfs::mkdir`), `scripts/qemu-smoke.sh` greps.
- Docs: this ADR, filesystem overview/stance, honesty ledger, roadmap note, threat-model v1.48, SUMMARY, light fr-nfr NFR-10 note.
- Follow-ups: EL0 `fs_mkdir` SVC + sample → [ADR-075](ADR-075-el0-fs-mkdir.md). Nested dirs + empty `rmdir` → [ADR-077](ADR-077-fat16-nested-rmdir.md). Recursive rm (non-goal unless sponsored) remains open. Thin TCP closed in [ADR-074](ADR-074-n5-thin-tcp.md).
