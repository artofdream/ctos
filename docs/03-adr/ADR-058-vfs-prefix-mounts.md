# ADR-058 — Thin VFS prefix mounts (virtual mounts)

- Status: Accepted (prefix-mount mile Verified when the serial / tests pass; Track A / app hosting still Planned for Linux/POSIX claims)
- Date: 2026-09-14

## Context

[ADR-027](ADR-027-thin-vfs-memfs.md) landed a thin VFS + memfs. [ADR-028](ADR-028-virtio-blk-fat16.md) added FAT16 behind the same `open`, with memfs-first then FAT fallback. Write / readdir / delete deepened that FAT backend ([ADR-050](ADR-050-fat16-write.md) / [ADR-056](ADR-056-fat16-readdir.md) / [ADR-057](ADR-057-fat16-delete.md)).

Docs “new vs extend” list ctos-specific **virtual mounts** (prefix / tree mounts; not a new on-disk format) as a later optional depth mile ([overview/filesystem.md](../overview/filesystem.md)). Routing today is an implicit overlay, not a mount table.

This mile is a **small mount table** that routes path **prefixes** to a backend. Keep A9 `/hello`, FAT write/readdir/delete, memfs create/write/read. Flat path grammar unchanged (`/` + `[a-z0-9_-]`, no extra `/`).

Not Linux VFS. Not POSIX `mount(2)` / mount namespaces / overlays. Not a new disk FS format.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Static prefix mount table; longest prefix wins. `/mem` (+ pinned A6 probe names) → memfs; `/` → FAT16. Serial `vfs: mount` / `vfs: mounts`. `#[test_case]` + fail-closed smoke. | Matches `/hello` / `/probe` on FAT and A6 memfs paths. No new SVC. |
| **H2 (rejected)** | Remap everything under `/fat/…` and `/mem/…` (tree mounts with an extra `/`). | Breaks flat path grammar and A9 `/hello`. |
| **H3 (rejected)** | Claim Linux `mount(2)` / vfsmount / mount namespaces. | Forbidden honesty ([ADR-034](ADR-034-linux-vfs-vs-thin-ctos.md) / ADR-031). |
| **H4 (rejected)** | Invent a new on-disk FS format for “virtual mounts.” | Overview forbids novel magic formats this mile. |
| **H5 (rejected)** | Keep memfs-first overlay only; document it as mounts without a table. | Not a real first cut; product-depth train wants an honest router. |

## Decision

1. **Mount table.** `vfs::MOUNTS`: `(prefix, backend)` entries. `vfs::resolve(path)` picks the **longest** matching prefix. First cut:
   - `/kprobe`, `/eprobe`, `/mwprobe`, `/mem` → **memfs** (A6 habits + documented `/mem` home)
   - `/` → **FAT16** (`/probe`, `/hello`, `/fwr`, `/fdel`, …)
2. **Route `create` / `open` / `unlink` by mount.** No memfs→FAT fallback overlay. `read` / `write` / `close` still key off the FAT fd tag. `readdir` stays FAT-root only (not a union listing).
3. **Probe.** Hello prints each `vfs: mount <prefix> -> <memfs|fat>` then `vfs: mounts`, then the existing `fs:*` memfs trip. Keep `fat:*` / `slot: ok`.
4. **No new SVC numbers.** Public ABI stays 16–23. Not POSIX `mount`.
5. **Fail-closed smoke.** Grep `vfs: mount` / `vfs: mounts` / keep `fs: ok` / `fat: ok` / `slot: ok`. `#[test_case]` covers resolve + memfs-mount create off the FAT volume.
6. **Honesty.** Say “the guest routed path prefixes through a thin VFS mount table” only when serial / tests pass. Do **not** say: Linux VFS, `mount(2)`, containers, POSIX, “supports FAT,” EL0 isolated, PAN, taken SError.
7. **NFR-10 text** is revised in place (ID unchanged). Threat-model **v1.33**. Do not mint NFR-15+.

## Consequences

- Code: `src/vfs.rs` (mount table, `resolve`, routed create/open/unlink, probe), `scripts/qemu-smoke.sh` greps.
- Docs: this ADR, [filesystem.md](../overview/filesystem.md), [framework/filesystem.md](../framework/filesystem.md), honesty ledger, roadmap note, threat-model v1.33, SUMMARY, architecture glance.
- Follow-ups (not this PR): tree mounts with an extra `/`, runtime mount/unmount SVC, memfs readdir, stripping the mount prefix inside backends.
