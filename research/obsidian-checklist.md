# Open this folder in Obsidian (one-page checklist)

Git markdown is the source of truth. Obsidian is an optional local viewer. Agents and CI never need it. Details: [obsidian.md](obsidian.md). Map of notes: [moc.md](moc.md).

This page is **structure**. It does not claim Obsidian is installed, synced, or used in CI. Do not commit `.obsidian/` or `.trash/`.

## First two minutes

1. Install Obsidian on *your* machine only if you want the UI (not in CI, not required for `gh` / agents).
2. **Open folder as vault** (Obsidian → Open folder as vault):
   - **repo root** (`ctos`) — sees `research/`, `docs/`, `AGENTS.md` (usual choice)
   - **`ctos/research`** — second-brain notes only
3. Pin [moc.md](moc.md) or this checklist in the left sidebar so the next session starts here.
4. Confirm the four vaults are ordinary folders (not a plugin):

| Vault | Folder | Job |
| --- | --- | --- |
| Procedure | `.cursor/skills/ctos-*/` | How to do a repeatable job |
| Correction | `.cursor/rules/` + `AGENTS.md` | Mistakes we must not repeat |
| Relationship | `docs/` | Vision → ADR → roadmap → ledger |
| Daily Brief | `research/daily-briefs/` | Where we stopped |

5. Fleeting captures go in [`inbox/`](inbox/). Copy [`templates/inbox-note.md`](templates/inbox-note.md). After a pass: `random-thoughts/`, `docs/`, or drop. Inbox is not the honesty ledger.
6. New handoff: copy [`templates/daily-brief.md`](templates/daily-brief.md) → `daily-briefs/YYYY-MM-DD-topic.md`. Scratch: [`templates/session-memory.md`](templates/session-memory.md) → `random-thoughts/`.
7. Confirm **`.obsidian/` is not staged** (`git status`). Same for `.trash/`. Both are gitignored on purpose (local workspace state).
8. No tokens, keys, or private account material in notes. Inbox, briefs, and docs are committed.
9. Do not enable Obsidian Sync / community plugins as a repo claim. A shared plugin list would be an explicit later commit — it is not done.

## Honesty

Opening the vault is optional. File presence here is not “Obsidian is installed.”
