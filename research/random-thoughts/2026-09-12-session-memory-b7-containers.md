# Session memory — 2026-09-12 (Track B / B7 containers non-goal)

Docs + `research/` only on `cursor/b7-containers-nongoal-6b58`. Parent `main` `ae7d2b8`.

- **B7 / #47:** make explicit that OCI/Docker guests are a **non-goal**. They need a Linux host. Today Docker only hosts the ctos smoke image.
- Honesty gaps that were still open-ended: Track B “far-later”; hosting-apps “not aiming at soon”; host-apps “on this horizon”; advantages grouping containers with “later” OTA.
- Landed short [ADR-029](../../docs/03-adr/ADR-029-containers-nongoal.md). Did not mint FR/NFR IDs. Did not implement a runtime.
- Avoided A8 surface: no sample-app rewrite, no `src/` change. Track A A8 stays Planned (#39).
- Reopen gate: new GitHub epic + new ADR. B1 must cite ADR-029.

Do not treat this file as the honesty ledger.
