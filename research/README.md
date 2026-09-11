# ctos second brain

Four vaults, scaled to a kernel repo. This is session memory in git — not a wiki and not a shop CRM.

| Vault | Where | Job |
| --- | --- | --- |
| Procedure | `.cursor/skills/ctos-*/` (playbooks) | How to do a repeatable job |
| Correction | `.cursor/rules/` + hard constraints in `AGENTS.md` (including PR identity: author ≠ merger) | Mistakes we must not repeat |
| Relationship | Docs cross-links (vision → architecture → ADR → roadmap → ledger) | How pieces connect |
| Daily Brief | `research/daily-briefs/` | Handoff: where we stopped |

Session scratch goes in `research/random-thoughts/`. Do not treat scratch as the honesty ledger.

Tracker is GitHub. These folders are the memory; `gh` is the work queue. Notes stay in this tree. A Route 53 CNAME `ctos.artof.link` → `artofdream.github.io.` exists; the custom domain is **Planned** until a separate Pages PR. Do not claim the URL works.

Idle tip on 2026-09-11: `main` ≈ `e80dc93` (Merge PR #28 / ADR-020). Latest handoff: [daily-briefs/2026-09-11-a9-os-app-slots.md](daily-briefs/2026-09-11-a9-os-app-slots.md) (A9 OS/app disconnect Planned). Prior: [scoped immutability](daily-briefs/2026-09-11-immutability-scoped.md).

## Further reading / landscape

Correction/Relationship notes (what to look for and against — not a status ledger):

- [Rust OS landscape lessons](random-thoughts/2026-09-09-rust-os-landscape-lessons.md) — Redox, Asterinas, Theseus, Tock, Hubris, phil-opp; stall patterns. No Verified claims about those projects.

## Obsidian (optional)

Git remains the source of truth. Obsidian is an optional local UI over the same markdown — not required for agents or CI.

- How to open the folder as a vault: [obsidian.md](obsidian.md)
- One-page open-vault checklist: [obsidian-checklist.md](obsidian-checklist.md)
- Map of content: [moc.md](moc.md)
- Templates: [templates/inbox-note.md](templates/inbox-note.md), [templates/daily-brief.md](templates/daily-brief.md), [templates/session-memory.md](templates/session-memory.md)
- Fleeting captures awaiting triage: [inbox/](inbox/)

