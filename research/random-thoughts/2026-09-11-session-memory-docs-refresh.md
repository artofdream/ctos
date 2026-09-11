# Session memory — 2026-09-11 (docs refresh after #21–#28)

First pass started from `main` `24d94e6`. Main moved to `e80dc93` (Merge PR #28 / ADR-020) while the draft was open. Rebased `cursor/docs-refresh-post-27-192e` onto that tip. Docs + `research/` only.

What we corrected on the rebase:

- ADR-020 is **Accepted / Verified** (not in-flight Planned). Serial `ident: reloc` / `ident: live`; `println!` after the tear.
- Sponsor Docker **Verified** on `e80dc93` (50 tests, `ident: reloc n=12`, live pages=37, force-fail). Kept the `24d94e6` Docker row.
- GHA merge [34651404108](https://github.com/artofdream/ctos/actions/runs/34651404108) grepped reloc/live/50 tests.
- Still Planned: `.rodata`/`.data`/heap tear, PAN, umbrella isolation.
- Pages: CNAME `ctos.artof.link` → `artofdream.github.io.` in `Z1178AFMV41RWP` / `737290977112` is real DNS config. Reachability stays Planned. This PR does not publish Pages.

Did not touch `src/`, `linker.ld`, or smoke scripts. Did not stage `.obsidian/` or `.trash/`.

Do not treat this file as the honesty ledger. Promote durable claims to `docs/`.
