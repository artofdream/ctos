# Daily brief — 2026-09-10 (pillars W^X / threat-model v1 / irq-delta / EL0 scaffold)

## Where we stopped

PR https://github.com/artofdream/ctos/pull/18 (`cursor/pillars-sec-perf-el0-9bc0`). Parent is `main` `0b1339d` (Merge PR #17, ADR-011). One coherent follow-up: P-SEC-1 v1, P-SEC-2 W^X, P-PERF irq-delta, P-SEC-3 EL0 stub, Obsidian checklist. No new FR/NFR IDs.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-10, QEMU 8.2.2, `rustc` 1.100.0-nightly `a36d05efa`): `Hello World!`, `paging: ok`, `heap: ok`, `sched: task a` / `sched: task b` / `sched: ok`, `wx: nx heap` / `wx: ok`, `perf: cntpct delta=22968`, `timer: tick`, `perf: irq-delta min=12973 max=35391 spread=22418 n=8`, `input: rx 0x41`, two `exception: sync BRK`, `exception: fatal nested` / `kind=0x200`, `Running 25 tests` all `[ok]`, force-fail exit 1.

GHA `smoke.yml` on this PR: **Unknown** (no green URL on the ledger-follow-up revision yet).

## Do next

1. Human or MRC review. Author does not merge (ADR-002). Merge hat: `artofdream` if GitHub shows `cursor[bot]`; `cursor[bot]` if the owner opened it.
2. EL0 implementation is **not** started (ADR-013 direction only).
3. Linker-stack NX / RO+NX text vs data / guard pages are later ADRs.

## Honesty

- Threat-model v1 is a file+review Verified. “Secure OS” unclaimed.
- Heap + coop stacks NX is Verified on this QEMU virt guest. Linker stacks are still executable.
- IRQ-delta is a spread, not a latency budget.
- cts-ai Docker Desktop path not re-run.
- `.obsidian/` was not committed.
