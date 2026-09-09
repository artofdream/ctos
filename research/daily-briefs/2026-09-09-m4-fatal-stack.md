# Daily brief — 2026-09-09 (M4 fatal exception stack)

## Where we stopped

PR https://github.com/artofdream/ctos/pull/7 (`cursor/m4-fatal-exception-stack-c8b7`). M4 / FR-07 only: `SP_EL0` thread stack, `SP_EL1` exception stack, fatal stack for nested current-EL, raw UART on that path, ADR-005.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-09, QEMU 8.2.2): `Hello World!`, two `exception: sync BRK esr=0xf2000000`, `exception: fatal nested` / `kind=0x200`, `Running 6 tests` all `[ok]`, force-fail exit 1. First hello run Failed (`MSR SP_EL1` UNDEF at EL1) — fixed in ADR-005.

GHA `smoke.yml` **Verified** on `ce1b4e5`: push [34390687124](https://github.com/artofdream/ctos/actions/runs/34390687124), PR [34390690943](https://github.com/artofdream/ctos/actions/runs/34390690943) (`ubuntu-24.04` + `ubuntu-24.04-arm`). Merged as PR #7 (`180dbf2`).

## Do next

1. M4 is on `main`. M5 (GIC + timer) already landed as PR #8 — do not restack it here.
2. Author does not merge (ADR-002).

## Honesty

- cts-ai offline. Docker Desktop path not re-run.
- Nested IRQ / lower-EL: Unknown (not taken).
- Probe is nested `BRK`, not an MMU stack-overflow fault (no paging yet).
