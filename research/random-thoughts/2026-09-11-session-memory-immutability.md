# Session memory — 2026-09-11 (scoped immutability)

Docs + `research/` only on PR #29.

Sponsor: is immutability compatible with ctos principles?

- **Yes, scoped:** RO code / later RO app images. Aligns with security + honesty + antifragility (probe the scope; keep Failed history).
- **Already practiced:** ADR-015 RO+NX + WXN; ADR-020 live `.text` tear after high-VA jump. Not “the kernel is immutable.”
- **No if absolute:** heap, PTEs, devices must mutate.
- Claim gate: only Verified where probed. No “immutable OS” marketing. No new ADR minted here.
- Track A [#31](https://github.com/artofdream/ctos/issues/31): RO app payloads after loader. Track B [#40](https://github.com/artofdream/ctos/issues/40): Linux-compat research must not inflate this.
- Sponsor clarification: the **goal** is to **disconnect OS updates from apps** (separate OS image vs app payloads). That is A9 [#48](https://github.com/artofdream/ctos/issues/48). **Planned after Track A ABI/loader.** Today still **one linked ELF — not Verified.** Overview now has an A9 section.

Page: `docs/framework/immutability.md`. Do not treat this file as the ledger.
