# Session memory — 2026-09-11 (docs plain English + mermaid)

Fetched `origin/main` `f86785b` (#30 merged). Branch `cursor/docs-plain-english-diagrams-e4a9`.

Probes this VM: `curl -sSI https://ctos.artof.link` HTTP 200; HTML has Driving principles; TLS CN=ctos.artof.link; Pages API cname + https_enforced + cert approved; `gh run view 34653046584` green; `dig CNAME` → `artofdream.github.io.`; github.io/ctos (no slash) 301 to custom domain; trailing-slash 404.

Rewrote landing to lead with what ctos is, principles, what runs, how to build. Glossed EL1/EL0/SVC/W^X/CNTPCT/PAN in everyday words before IDs. Ten mermaid fences (preprocessor + jsDelivr mermaid 11.6.0). Did not add `principles.md` (avoid SoT fork with #29).

`additional-css`/`additional-js` paths must be `docs/mermaid.css` (relative to book.toml), not bare filenames.

No kernel edits. Do not self-merge.
