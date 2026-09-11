# Session memory — 2026-09-11 (filesystem stance)

Docs + `research/` only on `cursor/docs-refresh-post-27-192e` (PR #29).

Sponsor asked for the same honesty as apps-today / porting:

- **No FS today.** `src/` has no VFS, memfs, virtio-blk, FAT, or inode code. Absence is the probe.
- **Planned order:** in-RAM memfs → virtio-blk → FAT *or* xv6-like (choose in the ADR that lands it; this note does not pick).
- Each step is one milestone / one PR. Do not mint FR-16+. POSIX `open` still waits on a later SVC ABI.
- Host `-drive` without guest code is not a probe. File presence of `docs/framework/filesystem.md` is not a guest `open`.

Ledger row: Filesystem **Planned**. Linked from README, overview, apps-today, building-or-porting, vision, architecture, moc.

Did not touch `src/` or smoke scripts. Do not treat this file as the honesty ledger.
