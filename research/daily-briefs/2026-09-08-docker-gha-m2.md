# Daily brief — 2026-09-08 (Docker / GHA / M2)

## Where we stopped

Same branch as the AArch64 retarget (`cursor/arm64-primary-isa-8dbd`, PR #4). Landed Docker smoke files, GHA `smoke.yml`, and M2 semihosting tests.

## Do next

1. Human or MRC review. Author does not merge (ADR-002).
2. Docker-on-cts-ai still Unknown (no Docker engine on the cloud VM).
3. Next kernel work after merge: M3 (VBAR), not more sensors unless a failure repeats.
4. Landscape memory (look for/against, not status): [rust-os-landscape-lessons](../random-thoughts/2026-09-09-rust-os-landscape-lessons.md).

## Honesty

- `scripts/qemu-smoke.sh` on this cloud VM: **Verified** (hello + cargo test 0 + force-fail 1).
- GHA `smoke.yml`: **Verified** (push 34285784458 + PR 34285786641 green; both `ubuntu-24.04` and `ubuntu-24.04-arm`).
- Docker engine: **Unknown**.
