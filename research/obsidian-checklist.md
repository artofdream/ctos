# Open this folder in Obsidian (one-page checklist)

Git markdown is the source of truth. Obsidian is an optional local viewer. Agents and CI never need it. Details: [obsidian.md](obsidian.md).

## Checklist

1. Install Obsidian on your machine (not in CI, not required for `gh` / agents).
2. **Open folder as vault.** Pick either:
   - the **repo root** (`ctos`) — sees `research/`, `docs/`, `AGENTS.md`
   - **`ctos/research`** — second-brain notes only
3. Confirm the four vaults are visible as ordinary folders: skills (Procedure), `.cursor/rules` + `AGENTS.md` (Correction), `docs/` (Relationship), `research/daily-briefs/` (Daily Brief). See [README.md](README.md).
4. Fleeting captures go in [`inbox/`](inbox/). Copy [`templates/inbox-note.md`](templates/inbox-note.md). Triage out of inbox; do not treat inbox as the honesty ledger.
5. Confirm **`.obsidian/` is not staged.** It is gitignored on purpose (local workspace state). Do not commit it. Same for `.trash/`.
6. Do not put tokens, keys, or private account material in notes. Inbox, briefs, and docs are committed.
7. Do not enable Obsidian Sync / community plugins as a repo claim. A shared plugin list would be an explicit later commit — it is not done.

## Honesty

This checklist is **structure**. It does not claim Obsidian is installed, synced, or used in CI.
