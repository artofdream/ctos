# Daily brief — 2026-09-12 (Track B / B2 Linux AArch64 syscall gap)

## Where we stopped

Ready PR https://github.com/artofdream/ctos/pull/65 (`cursor/b2-linux-syscall-gap-c35b`) rebased onto `main` `522c543` (#63 mobile theme). One docs-only PR: [linux-aarch64-syscall-gap.md](../../docs/research/linux-aarch64-syscall-gap.md) maps Linux AArch64 syscalls vs ctos SVC (reserved 0–2, public 16–23). No new ADR (not decision-grade; B6 is the decision). No new FR/NFR IDs. No B3–B6 implementation. No `src/` change. Ledger kept #63 mobile rows **and** B2 rows.

B2 is **Documented** on [track-b.md](../../docs/04-roadmap/track-b.md). Scratch: [random-thoughts/2026-09-12-session-memory-b2-syscall-gap.md](../random-thoughts/2026-09-12-session-memory-b2-syscall-gap.md).

## Do next

1. Separate MRC session already left `COMMENT` on #65. Author does not merge (ADR-002). GitHub lists this PR as `artofdream`-opened; merge hat is `cursor[bot]` after this-tip grepped checks and Bugbot resolved-or-declined. #63 merged; #64 still open and still shares the ledger.
2. Close [issue #42](https://github.com/artofdream/ctos/issues/42) after merge (`Closes #42` is on the PR).
3. B3 ([#43](https://github.com/artofdream/ctos/issues/43)) is process model vs `fork`/`exec`/`wait`. Do not add those syscalls.

## Honesty

- Docs + file-read of `src/syscall.rs` + Linux v6.10 `asm-generic/unistd.h` only. Did not run QEMU. Did not claim Linux userspace, a Linux ABI, or a new Pages deploy.
- **No Linux syscall is `present`.** Related Track A SVCs are **partial**. Namespace/mount are **never-per-ADR-031**.
- Local docs-build **Verified** on this cloud VM (mdBook 0.5.4 + mermaid 0.17.1; `docs-build: ok`; `book/CNAME` = `ctos.artof.link`; generated gap + track-b pages include **never-per-ADR-031**). Generator only.
- GHA on `60478ac` **Verified** (this authoring session grepped): smoke [34701938481](https://github.com/artofdream/ctos/actions/runs/34701938481) both matrices `slot: ok` + A1–A8 + 82 tests + `qemu-smoke: ok`; Pages [34701938331](https://github.com/artofdream/ctos/actions/runs/34701938331) `mdBook build` success (deploy skipped). Later ledger SHAs are not that probe.
- Ledger: inspection + local docs-build + GHA rows. Existing tables not rewritten. “Guest runs host apps” stays **Planned**.
