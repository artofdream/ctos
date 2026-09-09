# Daily brief — 2026-09-09 (M4 fatal exception stack)

## Where we stopped

PR on `cursor/m4-fatal-exception-stack-c8b7`. M4 / FR-07 only: `SP_EL0` thread stack, `SP_EL1` exception stack, fatal stack for nested current-EL, raw UART on that path, ADR-005.

Hello kernel: healthy-stack `BRK` (M3 string) then near-empty thread SP + nested `BRK` (fatal marker). `scripts/qemu-smoke.sh` greps both. GIC / timer is M5 — not in this PR.

## Do next

1. Cloud `qemu-smoke.sh` + GHA `smoke.yml` — record Verified or leave Unknown.
2. Human or MRC review. Author does not merge (ADR-002). Merge hat: `artofdream` if this PR is `cursor[bot]`; otherwise `cursor[bot]` after green checks.
3. After merge: M5 is GIC + timer (FR-08).

## Honesty

- cts-ai offline. Docker Desktop path not re-run.
- Nested IRQ / lower-EL: Unknown (not taken).
- Probe is nested `BRK`, not an MMU stack-overflow fault (no paging yet).
