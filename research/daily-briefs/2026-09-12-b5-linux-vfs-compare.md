# Daily brief — 2026-09-12 (Track B / B5 Linux VFS vs thin ctos VFS)

## Where we stopped

Ready PR https://github.com/artofdream/ctos/pull/66 (`cursor/b5-linux-vfs-vs-thin-ctos-f294`) rebased onto `main` `d81a53a` (#67 B3 process model = [ADR-035](../../docs/03-adr/ADR-035-process-model-standing-el0.md); #68 B4 ELF = [ADR-033](../../docs/03-adr/ADR-033-linux-elf-auxv-pt-interp.md)). One docs-only PR: [ADR-034](../../docs/03-adr/ADR-034-linux-vfs-vs-thin-ctos.md) compares Linux VFS (dentry / inode / path / mount) to the Track A thin VFS. Filename **ADR-034 is frozen**. Do not yield again. Cites [ADR-027](../../docs/03-adr/ADR-027-thin-vfs-memfs.md), [ADR-028](../../docs/03-adr/ADR-028-virtio-blk-fat16.md), [ADR-031](../../docs/03-adr/ADR-031-linux-compat-goals.md), and the B2 gap map (`openat` / `read` / `write` / `close` stay **partial**). Explicit **not claiming a Linux filesystem**. No new FR/NFR IDs. Ledger is append-only (B3 + B4 rows from main kept).

B3 / B4 / B5 are **Documented** on [track-b.md](../../docs/04-roadmap/track-b.md). Scratch: [random-thoughts/2026-09-12-session-memory-b5-linux-vfs.md](../random-thoughts/2026-09-12-session-memory-b5-linux-vfs.md).

## Do next

1. Coordinator merges after this-run green checks and Bugbot resolved-or-declined. Author does not merge (ADR-002). GitHub lists this PR as `artofdream`-opened; merge hat is `cursor[bot]`.
2. Close [issue #45](https://github.com/artofdream/ctos/issues/45) after merge (`Closes #45` is on the PR).
3. B6 is the decision. Do not grow POSIX flags / dentries.

## Honesty

- Docs + file-read of `src/vfs.rs` / `src/fat.rs` + B2 gap table only. Did not run QEMU. Did not claim Linux userspace, a Linux filesystem, or a new Pages deploy.
- **No Linux VFS object is `present`.** Path strings and handles are **partial**. Mount / overlay stay **never-per-ADR-031**.
- Local docs-build **Verified** after rebase onto `d81a53a` (mdBook 0.5.4 + mermaid 0.17.1; `docs-build: ok`; `book/CNAME` = `ctos.artof.link`; generated ADR-033 + ADR-034 + ADR-035 present; track-b B3/B4/B5 **Documented**). Generator only.
- Ledger: B3 + B4 rows from `d81a53a` / `774c672` + B5 inspection + prior-SHA GHA. Existing tables not rewritten. “Guest runs host apps” stays **Planned**.
- Rebase-tip `0cd62c2` smoke **Verified** (onto `774c672`): push [34714163309](https://github.com/artofdream/ctos/actions/runs/34714163309) + PR [34714165232](https://github.com/artofdream/ctos/actions/runs/34714165232). This-tip GHA after rebase onto `d81a53a` is Unknown until grepped.
