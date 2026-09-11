# Daily brief — 2026-09-11 (layout L3 / Docker paging miss)

## Where we stopped

#21 merged as `71ee15f` on `main`. Layout fix is on the tip. Follow-up branch `cursor/layout-gha-ledger-c915` only folds the GHA URLs.

cts-ai Docker on `b2bbb99`: **Failed** (sponsor serial). Cloud qemu-smoke on `c04b84b`: **Verified**. GHA on `c04b84b`: **Verified** — push [34563006005](https://github.com/artofdream/ctos/actions/runs/34563006005), PR [34563008516](https://github.com/artofdream/ctos/actions/runs/34563008516), both matrices grepped `paging: layout` / `paging: ok` / `el0: ok`.

Docker after the fix (`71ee15f`): **Unknown** until the sponsor re-runs `./scripts/docker-smoke.sh`.

## Do next

1. Sponsor re-probes cts-ai Docker Desktop on `main` `71ee15f`. This cloud VM has no Docker engine.
2. Isolation stays Planned. Do not claim “EL0 isolated.”
3. Identity image is W^X on this virt guest only. Do not say “secure OS.”
4. GitHub author of #21 was `artofdream`. ADR-002: that login does not merge its own PR.

## Honesty

- Docker Failed row is the sponsor serial on `b2bbb99`, not this VM.
- Cloud Verified is this QEMU virt guest on the dual-L3 ratchet layout.
- Did not pin an old nightly.
