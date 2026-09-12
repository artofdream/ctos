# Daily brief — 2026-09-12 (mdBook mobile / narrow viewport)

## Where we stopped

Draft-then-ready PR https://github.com/artofdream/ctos/pull/63 (`cursor/docs-mobile-responsive-81c4`). Parent is `main` `ba6541c` (A9 / ADR-030). One PR: CSS/theme only. No kernel change. No new FR/NFR IDs (FR-14 / NFR-06 / NFR-13).

Stock mdBook 0.5.4 already had viewport, hamburger sidebar, table wrappers, `img { max-width: 100% }`. This PR adds `docs/theme.css` (pre overflow, table-cell wrap, 44px tap targets ≤768px) and keeps Mermaid readable by scrolling instead of squashing.

## Honesty

- `./scripts/docs-build.sh` **Verified** on this VM (mdBook 0.5.4, mermaid 0.17.1).
- `./scripts/docs-mobile-probe.sh` **Verified** on this VM (`Google Chrome 148.0.7778.96`): landing overflow=no at 320 / 768 / 1280; ledger + `website.html` overflow=no at 320. Iframe forces the chapter width. Mermaid CDN blocked for the numeric run.
- Not a claim that `https://ctos.artof.link` is mobile-verified until a post-merge Pages fetch.
- Author does not merge (ADR-002).

## Do next

1. Separate MRC session (`COMMENT` only).
2. After merge + Pages deploy: re-fetch the live URL at a narrow width if you want a published-site mobile row.
