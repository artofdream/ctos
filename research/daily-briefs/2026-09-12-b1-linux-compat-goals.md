# Daily brief — 2026-09-12 (Track B / B1 Linux-compat goals)

## Where we stopped

Draft/ready PR https://github.com/artofdream/ctos/pull/62 (`cursor/b1-linux-compat-goals-9374`) off `main` `ba6541c` (A9 / #60). One docs-only PR: [ADR-031](../../docs/03-adr/ADR-031-linux-compat-goals.md) frames Linux-compat as ABI-**subset** research. Cites [ADR-029](../../docs/03-adr/ADR-029-containers-nongoal.md). Explicit **not claiming Linux userspace yet**. No new FR/NFR IDs. No B2–B6 implementation.

B1 is **Documented** on [track-b.md](../../docs/04-roadmap/track-b.md). Scratch: [random-thoughts/2026-09-12-session-memory-b1-linux-compat.md](../random-thoughts/2026-09-12-session-memory-b1-linux-compat.md).

## Do next

1. Separate MRC session (`COMMENT` only). Author does not merge (ADR-002). This cloud PR is `cursor[bot]`-authored; merge hat is `artofdream`.
2. Close [issue #41](https://github.com/artofdream/ctos/issues/41) after merge (`Closes #41` is on the PR).
3. B2 ([#42](https://github.com/artofdream/ctos/issues/42)) is the syscall gap map. Do not add Linux syscall numbers to `src/` in that child.

## Honesty

- Docs + file-read + local `./scripts/docs-build.sh` only. Did not run QEMU. Did not claim Linux userspace, a Linux ABI, or a new Pages deploy.
- Local docs-build **Verified** on this cloud VM (mdBook 0.5.4 + mermaid 0.17.1; `docs-build: ok`; `book/CNAME` = `ctos.artof.link`; generated ADR-031 + track-b pages include the frame sentence). Generator only.
- Ledger: one new document-inspection row + one docs-build row. Existing tables not rewritten. “Guest runs host apps” stays **Planned**.
