# Daily brief — 2026-09-09 (M9 cooperative scheduler)

## Where we stopped

PR for `cursor/m9-cooperative-scheduler-7da9`. Parent is `main` `a1b47d1` (Merge PR #14, M8 / FR-10). M9 / FR-11 only: cooperative EL1 yield, two heap-backed workers, serial `sched: task a` / `sched: task b` / `sched: ok`, ADR-010.

Preemption / SMP / EL0 are not this PR.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-09, QEMU 8.2.2): `Hello World!`, `paging: ok`, `heap: ok`, `sched: task a`, `sched: task b`, `sched: ok`, `timer: tick`, `input: rx 0x41`, two `exception: sync BRK`, `exception: fatal nested` / `kind=0x200`, `Running 19 tests` all `[ok]`, force-fail exit 1.

GHA `smoke.yml` **Verified** on the cloud-probe commit `88a9305`: push [34395786658](https://github.com/artofdream/ctos/actions/runs/34395786658) and PR [34395791288](https://github.com/artofdream/ctos/actions/runs/34395791288) (`ubuntu-24.04-arm` + `ubuntu-24.04`).

## Do next

1. Human or MRC review. Author does not merge (ADR-002).
2. Bugbot on this SHA was still in progress when GHA went green — wait until resolved-or-declined before merge.

## Honesty

- cts-ai may be online; prefer cloud/GHA probes. Docker Desktop path not re-run.
- Preemption / SMP / EL0: Unknown (not this path).
