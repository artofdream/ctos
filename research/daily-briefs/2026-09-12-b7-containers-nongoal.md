# Daily brief — 2026-09-12 (Track B / B7 containers non-goal)

## Where we stopped

Draft/ready PR https://github.com/artofdream/ctos/pull/58 (`cursor/b7-containers-nongoal-6b58`). Rebased onto `main` `e347983` (#50 Darwin docs-build pin) after a ledger conflict. One docs-only PR: [ADR-029](../../docs/03-adr/ADR-029-containers-nongoal.md) records that guest OCI/Docker/k8s is a **non-goal**. No new FR/NFR IDs. No container runtime.

Honesty gap closed: Track B no longer says containers are “far-later” or **Planned**. B7 is **Documented**. Site SoT: [hosting-apps.md](../../docs/overview/hosting-apps.md). Extra: [host-apps.md](../../docs/framework/host-apps.md). Ledger absence row still **Verified**; notes cite ADR-029. Scratch: [random-thoughts/2026-09-12-session-memory-b7-containers.md](../random-thoughts/2026-09-12-session-memory-b7-containers.md).

## Do next

1. Separate MRC session (`COMMENT` only). Author does not merge (ADR-002). This cloud PR is `cursor[bot]`-authored; merge hat is `artofdream`.
2. Close [issue #47](https://github.com/artofdream/ctos/issues/47) after merge (MRC / owner).
3. B1 ([#41](https://github.com/artofdream/ctos/issues/41)) must cite ADR-029 and must not reopen containers as Planned.

## Honesty

- Docs + source-absence probe only. Did not run QEMU. Did not claim a guest container runtime or a new Pages deploy.
- Local `./scripts/docs-build.sh` **Verified** on this cloud VM (mdBook 0.5.4 + mermaid 0.17.1; `docs-build: ok`; `book/CNAME` = `ctos.artof.link`). Generator only.
- Pages GHA on `1dd0a4f` **Verified**: [34688560928](https://github.com/artofdream/ctos/actions/runs/34688560928) (`mdBook build` 7s; deploy skipped). Smoke GHA also passed on that SHA (docs-only; not a new kernel mile). Later ledger SHAs are not that probe.
- Host `docker-smoke.sh` remains a harness (Docker hosts ctos), not the reverse.
