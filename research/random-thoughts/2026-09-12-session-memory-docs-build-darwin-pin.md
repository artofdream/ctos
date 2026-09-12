# Session memory — 2026-09-12 (docs-build Darwin + pin rebase)

#50 was stuck on `cursor/docs-build-host-os-c371` @ `3c84007` vs `main` `f86785b` (CONFLICTING). MRC asked to rebase onto current main and fold Darwin triples for **mdBook and mermaid** plus `${have#v}`. Do not revert mermaid. Do not republish the stale Pages Unknown.

Latest main at start of this rebase: `ae7d2b8` (A7 / ADR-028). Work on `cursor/docs-build-darwin-pin-4116`. Prefer updating #50’s branch if push works; otherwise a replacement PR and close #50.

GitHub release API listed Darwin tarballs for mdBook `v0.5.4` and mdbook-mermaid `v0.17.1`. This VM is Linux. Do not claim Darwin install Verified.

Do not treat this file as the honesty ledger.
