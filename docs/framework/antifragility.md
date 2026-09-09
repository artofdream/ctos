# Antifragility SOP

First-class pillar ([ADR-011](../03-adr/ADR-011-three-pillars.md), [NFR-05](../02-requirements/fr-nfr.md)). Hub: [pillars.md](pillars.md).

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

## No self-approval / no self-merge

The author does not `APPROVE` or merge their own PR. Same GitHub login is not a second identity. MRC writes `COMMENT` and names author / reviewer / merger. Merge hat is the *other* identity (`artofdream` vs `cursor[bot]`). See [ADR-002](../03-adr/ADR-002-pr-identity-split.md) and `.cursor/skills/ctos-mr-coordinator/SKILL.md`. Do not enable GitHub author self-APPROVE to make the gate “count.”
