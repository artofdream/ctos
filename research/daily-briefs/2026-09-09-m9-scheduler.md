# Daily brief — 2026-09-09 (M9 cooperative scheduler)

## Where we stopped

PR for `cursor/m9-cooperative-scheduler-7da9`. Parent is `main` `a1b47d1` (Merge PR #14, M8 / FR-10). M9 / FR-11 only: cooperative EL1 yield, two heap-backed workers, serial `sched: task a` / `sched: task b` / `sched: ok`, ADR-010.

Preemption / SMP / EL0 are not this PR.

Cloud `scripts/qemu-smoke.sh` and GHA: **Unknown** until probed on this branch.

## Do next

1. Probe qemu-smoke on this cloud VM; record Verified or leave Unknown.
2. Human or MRC review. Author does not merge (ADR-002).

## Honesty

- cts-ai may be online; prefer cloud/GHA probes. Docker Desktop path not re-run.
- Preemption / SMP / EL0: Unknown (not this path).
