# Antifragility SOP

When the same failure happens twice, **strengthen the strongest layer** (a sensor or a gate), not another paragraph of advice.

## Fail closed

- Unprobed boot → Unknown, not Verified.
- Missing nightly / rust-src / QEMU → say the environment blocked the probe; do not invent success.
- MRC / author conflict → do not merge.

## Ratchet

1. Name the failure once in `research/random-thoughts/` (what broke, command, error).
2. If it repeats, add a **sensor**: a script, a `#[test_case]`, a CI check, or a ledger row with a real probe command.
3. If people keep skipping the sensor, add a **gate**: CI required, or MRC refuses the PR.
4. Only then tighten a guide (`AGENTS.md` or a skill). Guides without sensors rot.

## No self-approval

The Kernel Engineer (or any author) does not mark their own PR Verified-to-merge. The MR Coordinator hat is a different job. See `.cursor/skills/ctos-mr-coordinator/SKILL.md`.
