# Daily brief — 2026-09-09 (M4 fatal exception stack)

## Where we stopped

PR https://github.com/artofdream/ctos/pull/7 (`cursor/m4-fatal-exception-stack-c8b7`). M4 / FR-07 only: `SP_EL0` thread stack, `SP_EL1` exception stack, fatal stack for nested current-EL, raw UART on that path, ADR-005.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-09, QEMU 8.2.2): `Hello World!`, two `exception: sync BRK esr=0xf2000000`, `exception: fatal nested` / `kind=0x200`, `Running 6 tests` all `[ok]`, force-fail exit 1. First hello run Failed (`MSR SP_EL1` UNDEF at EL1) — fixed in ADR-005.

GHA `smoke.yml` still **Unknown** until a run URL on the post-fix commit.

## Do next

1. Wait for GHA; record Verified or leave Unknown.
2. Human or MRC review. Author does not merge (ADR-002). Merge hat: `artofdream` if GitHub shows `cursor[bot]`; `cursor[bot]` if the owner opened it.
3. After merge: M5 is GIC + timer (FR-08).

## Honesty

- cts-ai offline. Docker Desktop path not re-run.
- Nested IRQ / lower-EL: Unknown (not taken).
- Probe is nested `BRK`, not an MMU stack-overflow fault (no paging yet).
