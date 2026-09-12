# ADR-027 — Thin VFS + in-RAM memfs

- Status: Accepted (memfs mile Verified when the serial / tests pass; Track A / app hosting still Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding app to read and write **named buffers**. [Issue #37](https://github.com/artofdream/ctos/issues/37) is this mile: a **thin VFS** with one backend, an in-RAM **memfs**, plus a fail-closed create / write / read / close probe.

[ADR-021](ADR-021-svc-syscall-abi.md) already has `exit` / `uart_write` / `yield` (16–18). There is no path, no handle, and no store. A later virtio-blk + on-disk FS (A7 / [issue #38](https://github.com/artofdream/ctos/issues/38)) must not invent a second “open” story. The VFS ADR lands first.

This is **not** Linux VFS. **Not** POSIX `open` / `stat` / `mount`. **Not** a disk. **Not** app hosting.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Tiny `VfsOps` (create / open / read / write / close) + one `MemFs` of heap `Vec<u8>` named files. New public SVC 19–23. Kernel probe + EL0 trampoline. | Same “easiest in-tree” shape as a coop task. No DMA. One backend this mile. |
| **H2 (rejected)** | Skip the trait and call memfs from SVC only. | A7 would then invent a second interface. The VFS *is* the mile. |
| **H3 (rejected)** | Ship virtio-blk or FAT in the same PR. | A7. Host `-drive` without guest code is not a probe. Do not say “supports FAT.” |
| **H4 (rejected)** | Linux/POSIX `open` flags, directories, cwd, `O_CREAT`. | Inflates the mile and invites a POSIX claim. |

## Decision

1. **Thin VFS.** `src/vfs.rs` defines `VfsOps`: `create` / `open` / `read` / `write` / `close` on a path + handle. One global backend this mile: `MemFs`. Later A7 may add a block FS behind the same trait. Not a superblock. Not a dentry cache.
2. **memfs.** Flat namespace. Paths are `/` plus 1–15 bytes of `[a-z0-9_-]` (no extra `/`, no `.`). At most 8 files, 256 bytes each, 4 handles. File bytes live on the first-fit heap (`Vec<u8>`, [ADR-009](ADR-009-first-fit-heap.md)). Handles are `1..=4`. Offset is per handle. `create` fails if the name exists. `open` fails if missing. `write` stores at the handle offset and grows up to `FILE_MAX`. `read` copies from the offset. `close` drops the handle; the file stays. Not a directory tree.
3. **Public SVC numbers (extends ADR-021).** Same calling convention (`SVC #<n>`, `x0`–`x2`, return `x0`). Reserved 0–2 stay ADR-013 probes.

   | Number | Name | Args | Effect | Return |
   | --- | --- | --- | --- | --- |
   | 19 | `fs_create` | `x0` = user path, `x1` = length | Create an empty named file and a handle | fd `>= 1`, or `0` |
   | 20 | `fs_open` | `x0` = user path, `x1` = length | Open an existing name | fd `>= 1`, or `0` |
   | 21 | `fs_read` | `x0` = fd, `x1` = user buf, `x2` = length | Copy from the handle offset into a user-mapped **and** kernel-mapped range | bytes, or `u64::MAX` on reject (`0` is a valid empty read) |
   | 22 | `fs_write` | `x0` = fd, `x1` = user buf, `x2` = length | Copy into the file at the handle offset | bytes, or `0` |
   | 23 | `fs_close` | `x0` = fd | Drop the handle | `0` on success, `u64::MAX` on reject |

   Path / I/O length is capped (path 16, I/O 64). Fail-closed: empty length, overflow, non-canonical / TTBR1 alias, or a range that is not user-mapped **or** not kernel-mapped returns the reject value and does not mutate the file. Kernel `.data` is rejected. Unknown immediates still park.
4. **`libctos` wrappers.** `fs_create` / `fs_open` / `fs_read` / `fs_write` / `fs_close` over `svc #19`–`#23`. The A2 hello payload stays `exit` / `uart_write` / `yield` only so A2–A4 markers do not change.
5. **Fail-closed probe.** Kernel create / write / read / close of `/kprobe` prints `fs: create` / `fs: write` / `fs: read`. An EL0 trampoline does the same for `/eprobe` and prints `fs: el0`. Then `fs: ok`. `scripts/qemu-smoke.sh` greps those and rejects `fs: probe missed`. `#[test_case]` covers the kernel round-trip, a missing `open`, the EL0 trip, and a kernel-`.data` write reject. Existing `svc:` / `libctos:` / `loader:` / `el0: task-*` / A5 `ident: rodata*` / `pan:*` markers stay.
6. **Honesty.** Say “the guest created, wrote, and read a named in-RAM file behind a thin VFS” only when the serial / tests pass. Do **not** say: app hosting is done, POSIX, Linux VFS, FAT, virtio-blk, on-disk FS, userspace, “EL0 isolated,” or “secure OS.” Track A A7–A9 stay Planned.
7. **NFR-10 text** is revised in place (ID unchanged) to name this memfs mile. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/vfs.rs`, `src/syscall.rs` (19–23), `libctos/` wrappers, `scripts/qemu-smoke.sh`.
- Docs: [filesystem.md](../framework/filesystem.md), [overview/filesystem.md](../overview/filesystem.md), [syscall.md](../framework/syscall.md), [track-a.md](../04-roadmap/track-a.md). Roadmap cites #37 / Track A #31.
- A7 (virtio-blk + FAT **or** xv6-like on that block device) is not this PR. Choose the on-disk format in that ADR.
