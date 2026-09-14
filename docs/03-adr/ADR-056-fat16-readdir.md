# ADR-056 — FAT16 readdir behind the thin VFS

- Status: Accepted (FAT16 root listing mile Verified when the serial / tests pass; Track A / app hosting still Planned for Linux/POSIX claims)
- Date: 2026-09-14

## Context

[ADR-028](ADR-028-virtio-blk-fat16.md) landed virtio-blk + FAT16 **read**. [ADR-050](ADR-050-fat16-write.md) deepened **write** / small create and deferred directory listing. Docs “new vs extend” prefer deepening the known small FS (FAT16) behind the thin VFS ([ADR-027](ADR-027-thin-vfs-memfs.md)).

This mile is guest **root directory listing**: walk FAT16 root dirents and return thin-VFS paths (`/probe`, `/hello`, `/fwr`). Same path grammar. Fail-closed serial + `#[test_case]`.

Not POSIX `getdents` / `opendir` / `readdir(3)`. Not a directory tree. Not delete/unlink. Not virtual mounts. Keep write, `/hello`, cross-update, `slot:ok`.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Kernel + thin VFS `vfs::readdir` lists FAT16 **root** file names as `/…` paths. Serial `fat: readdir` / `fat: entries n=<k>`. Expect `/probe`, `/hello`, `/fwr` after the write-mile create. No new SVC this mile. | Deepens ADR-028/050. Same path grammar. Caps: `READDIR_MAX` (16). |
| **H2 (rejected)** | Claim POSIX `getdents` / `opendir` or add Linux dirent ABI. | Forbidden honesty. |
| **H3 (rejected)** | New `SVC` / `libctos` wrapper this PR. | Write mile used existing `vfs::write`. Listing is a kernel/VFS depth probe; EL0 SVC can follow later. |
| **H4 (rejected)** | Subdirectory trees, LFN, delete/unlink, or virtual mounts. | Out of scope; follow-ups. |
| **H5 (rejected)** | Route memfs through the same listing as a union namespace. | memfs stays flat named buffers; this mile is FAT16 root only. |

## Decision

1. **`fat::readdir` / `vfs::readdir`.** Walk the FAT16 root, skip deleted / volume / LFN / directory entries, map 8.3 (blank extension) to thin-VFS paths via the inverse of `path_to_83`. Cap `READDIR_MAX`. Overflow → `FsError::Full` (fail-closed).
2. **Probe after create.** `observe_probe` lists the root after `/fwr` exists so the write-mile name is visible. Serial `fat: readdir` then `fat: entries n=<k>`. Require `/probe`, `/hello`, `/fwr`. Keep `fat: ok` / write / A9 slot markers.
3. **No new SVC numbers.** Public ABI stays 16–23. Not a POSIX product claim.
4. **Fail-closed smoke.** Grep `fat: readdir` / `fat: entries` / keep `fat: ok`. `#[test_case]` covers listing + cap Full.
5. **Honesty.** Say “the guest listed FAT16 root directory entries through the thin VFS” only when serial / tests pass. Do **not** say: POSIX `getdents`/`opendir`, FAT32, “supports FAT,” EL0 isolated, PAN, taken SError, Linux, containers.
6. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.31**. Do not mint NFR-15+.

## Consequences

- Code: `src/fat.rs` (list_root / readdir / probe), `src/vfs.rs` (`vfs::readdir`), `scripts/qemu-smoke.sh` greps.
- Docs: this ADR, [filesystem.md](../overview/filesystem.md), [framework/filesystem.md](../framework/filesystem.md), honesty ledger, roadmap note, threat-model v1.31, SUMMARY.
- Follow-ups (not this PR): EL0 `fs_readdir` SVC, LFN, subdirectories, virtual mounts, memfs listing. Delete/unlink: [ADR-057](ADR-057-fat16-delete.md).
