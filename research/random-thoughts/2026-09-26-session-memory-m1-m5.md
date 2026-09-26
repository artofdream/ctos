# Session memory — 2026-09-26 M1–M5 (after sponsor D1–D5)

- Stack (merge order, sponsor merges, ADR-002): #136 ADR-084 → #137 ADR-085 → #138 ADR-086 → #139 ADR-087 (M1) → #140 ADR-088 (M2) → #141 ADR-089 (M3) → #142 ADR-090 (M4) → #143 ADR-091 (M5 draft).
- Identity inventory: 5 kernel ranges → 2 (M1) → 1 (M2, stub only). User 2 → 1.
- Gotchas:
  - `.rodata` identity-pointer rewrite relocates constant tables that hold 0x4008_0000-like words. Compare `identity_pa()` bounds, never raw words.
  - A GitHub job skipped via `if:` passes a required check, so the pin job had to become unconditional before D5.
  - EL0 exec permission is governed by UXN, not AP[1]. The stub page is safe because every kernel builder sets UXN.
- Commit path: box patch → CopyFromBox (lands at C:\Users\cts\) → EVO-X2 `git apply --index` + write-tree compare → push; open PRs from box `gh api`.
- Open for the sponsor: accept ADR-091 (a/b/c), G1/G2 probes, optional B2-P re-verify ≈ $0.2, D5 branch protection at M6.
