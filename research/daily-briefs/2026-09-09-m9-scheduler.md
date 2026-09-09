# Daily brief — 2026-09-09 (M9 cooperative scheduler)

## Where we stopped

M9 / FR-11 is on `main` (`4e3b732`, merge of PR #15). Cooperative EL1 yield, two heap-backed workers, serial `sched: task a` / `sched: task b` / `sched: ok`, ADR-010.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-09, QEMU 8.2.2): `Hello World!`, `paging: ok`, `heap: ok`, `sched: task a`, `sched: task b`, `sched: ok`, `timer: tick`, `input: rx 0x41`, two `exception: sync BRK`, `exception: fatal nested` / `kind=0x200`, `Running 19 tests` all `[ok]`, force-fail exit 1.

GHA `smoke.yml` **Verified**:
- Cloud-probe commit `88a9305`: push [34395786658](https://github.com/artofdream/ctos/actions/runs/34395786658), PR [34395791288](https://github.com/artofdream/ctos/actions/runs/34395791288)
- Merge commit `4e3b732`: [34396135202](https://github.com/artofdream/ctos/actions/runs/34396135202)

GitHub author of #15 was `artofdream`. ADR-002 merge hat was `cursor[bot]`; owner merged. Same hat miss as earlier milestone PRs. Bugbot was still in progress at merge.

## Do next

1. Land this docs follow-up so `main`’s ledger cites the GHA URLs (row was still Unknown at merge).
2. Preemption / SMP / EL0 are not started.
3. #14 heap-align follow-up may still be open; not this PR.

## Honesty

- cts-ai Docker Desktop path not re-run.
- Preemption / SMP / EL0: Unknown (not this path).
