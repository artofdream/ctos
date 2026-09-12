# ADR-034 — Linux VFS concepts vs thin ctos VFS

- Status: Accepted (docs compare; no Linux FS claim)
- Date: 2026-09-12
- Number: **ADR-034 is frozen for this mile.** Main holds ADR-033 (B4 ELF / auxv / `PT_INTERP`, #68 / `774c672`) and ADR-035 (B3 process model, #67 / `d81a53a`). Do not yield this number again.

## Context

Track B ([issue #40](https://github.com/artofdream/ctos/issues/40)) is Linux-compat **research**. Child [B5 / #45](https://github.com/artofdream/ctos/issues/45) asks how ctos VFS / memfs / FAT stays **thin** while optionally mapping to Linux-like semantics later.

[ADR-031](ADR-031-linux-compat-goals.md) already framed Linux-compat as ABI-**subset** research and told this child: compare Linux VFS concepts vs the thin ctos VFS; **do not grow POSIX `open` flags / dentries**. [B2](../research/linux-aarch64-syscall-gap.md) already mapped the file-descriptor slice: Linux AArch64 `openat` / `read` / `write` / `close` are **partial** against ctos `fs_create` / `fs_open` / `fs_read` / `fs_write` / `fs_close` (SVC 19–23). Linux `mount` / `umount2` / `pivot_root` stay **never-per-ADR-031**.

Track A first cuts stay freestanding:

- Thin VFS + in-RAM memfs ([ADR-027](ADR-027-thin-vfs-memfs.md)): `VfsOps` is create / open / read / write / close. Flat path `/` + 1–15 of `[a-z0-9_-]`. At most 8 files, 256 bytes, 4 handles.
- virtio-blk + read-only FAT16 behind the **same** `vfs::open` ([ADR-028](ADR-028-virtio-blk-fat16.md)). Known files `/probe` and A9 `/hello`. Guest `write` on a FAT handle is `ReadOnly`. Serial `fat: mount` is a probe marker, not `mount(2)`.

This ADR does **not** implement B3, B4, or B6. It does **not** mint FR-16+ or NFR-15+. It does **not** add POSIX flags, dentries, cwd, or a second `open` story to `src/`. File presence of this page is not a Linux filesystem.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Compare Linux VFS objects (dentry / inode / path / mount) to today’s thin VFS. Stay thin. Document gaps. Optional later mapping is translation **after** B6, not a dentry cache this mile. | Matches #45 / ADR-031. Leaves B6 room to choose **never**. |
| **H2 (rejected)** | Grow POSIX `open` flags, dentries, cwd, or `O_CREAT` in this PR. | ADR-031 forbids it. Inflates Track A probes and invites a POSIX claim. |
| **H3 (rejected)** | Say “ctos has Linux VFS” because memfs + FAT16 exist. | Honesty fail (NFR-06). Serial `fs: ok` / `fat: ok` are thin-VFS probes, not Linux VFS. |
| **H4 (rejected)** | Add `mount(2)`, overlay, or a mount namespace as Planned on Track B. | Conflicts with ADR-031 / [ADR-029](ADR-029-containers-nongoal.md). A7 FAT attach is not a Linux mount tree. |
| **H5 (rejected)** | Decide compat layer vs reimplement vs never here. | That is [B6 / #46](https://github.com/artofdream/ctos/issues/46). |

## Decision

1. **Stay thin.** `VfsOps` remains five calls on a path + handle. One `open` story. memfs first; if the name is missing, FAT16 root (8.3) is tried. FAT fds are tagged so the router does not invent a second interface. Not a superblock. Not a dentry cache. Not POSIX `open`.

2. **What Linux VFS is (concepts, not a port).** Linux path lookup is a walk of cached **dentries** that bind a name to an **inode**, under a **vfsmount**, on a **super_block**. An open file is a separate **file** object (`f_pos`, `f_op`). A **path** is the pair `(mnt, dentry)`. Hard links share an inode across dentries. `.` / `..` / extra `/` / relative names / `AT_FDCWD` / symlinks are part of that walk. This paragraph is a teaching sketch of Linux, not a claim that ctos implements any of it.

3. **Concept map.** Status words match B2: **present** means the Linux object exists here with comparable semantics; **partial** means a related ctos object exists but identity or walk still differ; **absent** means no analog; **never-per-ADR-031** means a Linux mount / overlay / namespace class already ruled out.

   **No Linux VFS object is `present`.**

   | Linux concept | ctos today | Status | Why not `present` |
   | --- | --- | --- | --- |
   | `super_block` (FS instance, `s_op`, root) | Two backends behind one router (`MemFs` + `Fat16`) | **absent** | No superblock, no FS type registration, no `s_root`. |
   | `inode` (identity, `i_ino`, `i_mode`, `i_op`) | memfs `File` (name + `Vec<u8>`) or FAT 8.3 dirent + cluster | **absent** | No inode number, mode, nlink, or inode ops. Close does not destroy the memfs bytes; that is not an inode lifetime. |
   | `dentry` (cached name → inode, `d_parent`) | Exact string match on a flat table | **absent** | No dentry cache, no parent, no hash walk, no RCU-walk. ADR-027 rejected a dentry cache. |
   | `path` (`vfsmount` + `dentry`) | Canonical byte string `/` + `[a-z0-9_-]` | **partial** | A path exists as a name. It is not a `(mnt, dentry)` pair. |
   | `file` (open instance, `f_pos`, `f_op`) | Handle `1..=4` with a per-handle offset | **partial** | Related effect to an fd + offset. Four handles. No `f_op`, no fd 0/1/2, no `dup`. FAT handles use a tag bit, not a Linux fd table. |
   | `vfsmount` / mount tree | Serial `fat: mount` after virtio + FAT attach | **never-per-ADR-031** | Not `mount(2)`. No mount namespace, no overlay, no `umount2` / `pivot_root`. B2 already marked those **never-per-ADR-031**. |
   | Directory / `getdents64` | Flat namespace; FAT root is a lookup table, not a listed dir | **absent** | No `mkdirat`, no cwd. |
   | `stat` / `statx` / inode metadata | None | **absent** | B2: `newfstatat` / `fstat` / `statx` **absent**. |
   | Symlink / hard link | None | **absent** | One name per memfs slot; FAT 8.3 one dirent. |
   | POSIX path walk (`.`, `..`, `//`, relative, `AT_FDCWD`) | `valid_path` rejects extra `/`, `.`, and anything outside `[a-z0-9_-]` | **absent** | See path grammar below. |

4. **POSIX path grammar vs ctos names.**

   | Item | Linux / POSIX | ctos (`valid_path`) |
   | --- | --- | --- |
   | Shape | Absolute or relative; `/` separators; empty components collapse | Must start with `/`; **no** extra `/` |
   | `.` / `..` | Current / parent | Rejected |
   | Length | `PATH_MAX` (typically 4096) | 2–16 bytes total (`/` + 1–15) |
   | Alphabet | Almost any byte except NUL and `/` | `[a-z0-9_-]` after the first `/` |
   | cwd / `chdir` / `getcwd` | Yes | No. B2: `chdir` / `getcwd` **absent** |
   | `dirfd` / `AT_FDCWD` | `openat` first argument | No. `fs_open` takes a user path pointer + length |
   | `O_*` / mode | `openat` flags + mode | No. Create is a **separate** SVC (`fs_create` 19). Existing name is `fs_open` 20. |
   | FAT 8.3 | `vfat` long names + short names | Guest path `/probe` maps to `PROBE`; `/hello` to `HELLO`. Not VFAT. |

5. **Backends are not Linux FS types.** memfs is named heap buffers, not `tmpfs`. FAT16 on virtio-blk is a host-built 4 MiB image with two known files, not Linux `vfat` / `msdos`. Do not say “supports FAT” as a product. Say “the guest read a known FAT16 file through the thin VFS” only when the ledger has `fat: ok`.

6. **Syscall slice stays on B2.** This ADR does not retarget SVC 19–23 to Linux `x8`. The B2 rows still stand: `openat` **partial**, `read` **partial**, `write` **partial** (`uart_write` **or** `fs_write`, neither is `write(1, …)`), `close` **partial**, `lseek` / `pread64` / `getdents64` / `ioctl` **absent**, `mount` **never-per-ADR-031**.

7. **Optional later mapping (not a decision).** If [B6](https://github.com/artofdream/ctos/issues/46) ever names a compat path, the *smallest* file slice would still be convention translation of `openat` / `read` / `write` / `close` onto the existing thin VFS — not a Linux dentry cache, not POSIX flags, not cwd, not `mount(2)`. Growing those objects needs a **later epic + ADR** after B6. **Never** remains valid.

8. **Not claiming a Linux filesystem.** Say that sentence on the Track B page. Today:

   - Thin VFS + memfs / read-only FAT16 is not Linux VFS.
   - `fat: mount` is not `mount(2)`.
   - A9 FAT `/hello` is not a Linux rootfs.
   - The ledger row “Guest runs host apps (Linux ELF / shell / Python)” stays **Planned**. This ADR does **not** change that row to Verified.

9. **Keep (ctos specificity).** Honesty (NFR-06): this file is not Verified Linux VFS. Antifragility keeps `fs:` / `blk:` / `fat:` smoke strings. Security: no “Linux-compat isolation” from a compare. Performance: no invented VFS bench. Freestanding Track A path stays. Guest containers stay [ADR-029](ADR-029-containers-nongoal.md).

## Consequences

- Docs: [track-b.md](../04-roadmap/track-b.md) B5 becomes **Documented**. [SUMMARY.md](../SUMMARY.md) and [roadmap.md](../04-roadmap/roadmap.md) link here. Site extra: [filesystem.md](../overview/filesystem.md) / [filesystem.md](../framework/filesystem.md) point at the compare without a Linux FS claim.
- B6 stays **Planned** research. B3 is **Documented** ([ADR-035](ADR-035-process-model-standing-el0.md)). B4 is **Documented** ([ADR-033](ADR-033-linux-elf-auxv-pt-interp.md)). No `src/` change in this PR.
- Track A A6/A7 probes stay the product path for named files. Do not grow `VfsOps` to look like Linux so a research ADR can say `present`.
- Honesty: document inspection of this ADR + `src/vfs.rs` / `src/fat.rs` + the B2 gap table. Do not add a Verified Linux-filesystem ledger row because this file exists.
