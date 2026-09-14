# ADR-057 — FAT16 delete behind the thin VFS

- Status: Accepted (FAT16 root delete mile Verified when the serial / tests pass; Track A / app hosting still Planned for Linux/POSIX claims)
- Date: 2026-09-14

## Context

[ADR-028](ADR-028-virtio-blk-fat16.md) landed virtio-blk + FAT16 **read**. [ADR-050](ADR-050-fat16-write.md) deepened **write** / small create. [ADR-056](ADR-056-fat16-readdir.md) deepened **root listing**. Docs “new vs extend” prefer deepening the known small FS (FAT16) behind the thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md)).

This mile is guest **root file delete**: mark a FAT16 root dirent deleted (`0xE5`), free its cluster chain, and expose the same path grammar through `vfs::unlink`. Fail-closed serial + `#[test_case]`.

Not POSIX `unlink` / `remove` / `rm`. Not a directory tree. Not virtual mounts. Keep write, readdir, `/hello`, cross-update, `slot: ok`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Kernel + thin VFS `vfs::unlink` deletes a FAT16 **root** file. Serial `fat: delete` / keep `fat: ok`. Probe creates `/fdel`, deletes it, confirms `Missing` + absent from `readdir`. No new SVC this mile. | Deepens ADR-028/050/056. Same path grammar. |
| **H2 (rejected)** | Claim POSIX `unlink` / `remove` or add a Linux unlink ABI. | Forbidden honesty. |
| **H3 (rejected)** | New `SVC` / `libctos` wrapper this PR. | Write/readdir miles used existing VFS; delete is a kernel/VFS depth probe; EL0 SVC can follow later. |
| **H4 (rejected)** | Subdirectory trees, LFN, multi-cluster shrink helpers, or virtual mounts. | Out of scope; follow-ups. |
| **H5 (rejected)** | Route only memfs through `vfs::unlink` this mile. | Product-depth train is FAT16; memfs unlink is incidental for the same entry point. |

## Decision

1. **`fat::unlink` / `vfs::unlink`.** Look up the 8.3 root dirent, mark first byte `0xE5`, free the cluster chain (walk to EOC; single-cluster files this mile), drop open handles to that name. Missing → `FsError::Missing` (fail-closed).
2. **Probe after readdir.** Create `/fdel`, write `fat-dl`, `vfs::unlink("/fdel")`, confirm open/`has_name` Missing and root listing still has `/probe`, `/hello`, `/fwr` without `/fdel`. Serial `fat: delete`. Keep `fat: ok` / write / readdir / A9 slot markers.
3. **No new SVC numbers.** Public ABI stays 16–23. Not a POSIX product claim.
4. **Fail-closed smoke.** Grep `fat: delete` / keep `fat: ok` / readdir / write. `#[test_case]` covers delete + missing.
5. **Honesty.** Say “the guest deleted a FAT16 directory entry through the thin VFS” only when serial / tests pass. Do **not** say: POSIX `unlink`/`remove`, FAT32, “supports FAT,” EL0 isolated, PAN, taken SError, Linux, containers.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.32**. Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (mark deleted / free chain / probe), `src/vfs.rs` (`vfs::unlink`), `scripts/qemu-smoke.sh` greps.
- Docs: this ADR, [filesystem.md](../overview/filesystem.md), [framework/filesystem.md](../framework/filesystem.md), honesty ledger, roadmap note, threat-model v1.32, SUMMARY.
- Follow-ups (not this PR): EL0 `fs_unlink` SVC, LFN, subdirectories, virtual mounts, richer samples.
