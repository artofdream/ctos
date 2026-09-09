# Obsidian (optional local UI)

Git markdown is the source of truth. Obsidian is an optional local viewer/editor over the same files. Agents and CI still read the markdown in this repo. Nothing here configures Obsidian sync, plugins, or a shared vault.

## Open a vault

In Obsidian: **Open folder as vault**, then pick either:

- the repo root (`ctos`) — sees `research/`, `docs/`, `AGENTS.md`
- `ctos/research` — second-brain notes only

Both are valid. The four vaults live in git either way; see [README.md](README.md).

## Why `.obsidian/` is gitignored

Obsidian writes local workspace state under `.obsidian/` (and discarded notes under `.trash/`). Those paths are in `.gitignore` on purpose so a local UI does not leak machine-specific config into the tree. That is **portability**, not a security ban. If you later want a shared plugin list, that is a separate, explicit commit — it is not done here.

## Do not put secrets in notes

Inbox, random-thoughts, daily briefs, and docs are committed. No tokens, keys, or private account material.

## Honesty

This change is **structure only**: an inbox folder, a thin template, and this how-to. It does not claim that Obsidian is installed, synced, or used in CI.
