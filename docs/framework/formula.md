# ctos harness map

A reliable kernel session is:

**Shared understanding + domain + outer harness.**

That three-part split is common harness-engineering language (prior art: [architecture.artof.link](https://architecture.artof.link/)). On ctos it is not a product name. It is a map for this repo.

## Shared understanding

The session's reviewable model of intent and state:

- Vision, architecture, ADRs, roadmap
- Honesty ledger (what we claim vs what we probed)
- Daily briefs and random-thoughts (handoff, not source of truth)

If a claim is only in chat, it is not shared understanding.

## Domain

The thing that is allowed to decide what is true about the machine:

- Kernel source and the custom AArch64 target
- QEMU `virt` (and later real hardware, if probed)
- Kernel ELF loaded by `-kernel`
- CPU, UART, interrupt, and memory behavior

Agents interpret. The domain (a build, a QEMU probe, a test) decides. Do not "confirm boot" from a README sentence.

## Outer harness (six layers, kernel-scaled)

| Layer | On ctos | Prevents |
| --- | --- | --- |
| Guides | `AGENTS.md`, `.cursor/rules/`, `.cursor/skills/ctos-*` | Out-of-scope work (wrong crate era, shop content, GitLab SOPs) |
| Sensors | `scripts/qemu-smoke.sh`, `cargo test` semihosting exit, GHA `smoke.yml`, optional Docker | Rounding "source exists" up to "it boots" |
| Loop | One milestone → one branch → one GitHub PR | Sprawling PRs that mix UART, paging, and docs rewrites |
| Memory | `research/` vaults | Session amnesia; stuffing raw chat into the next prompt |
| Permissions | Author ≠ merger (`artofdream` vs `cursor[bot]`); MRC writes COMMENT; no GitHub self-APPROVE (ADR-002) | Same-login stamp counted as a second review |
| Observability | Honesty ledger + PR text that lists probed vs Unknown | Status theater |

Fail closed: if QEMU was not run, boot status is Unknown. Do not write Verified.
