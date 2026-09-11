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

## Recent ratchets (keep Failed history)

These are **history rows**, not a claim that the current tip is broken.

- **cts-ai Docker on `b2bbb99` Failed** after PR #20 (`paging: probe missed` / `heap: probe missed` / `sched: probe missed` / `el0: probe missed` while guard/RO still printed). Same class as the PR #20 pre-MMU `.bss` miss plus a single shared L3 / first-2-MiB user map. **Sensor:** layout L3 pool + post-MMU frame init (#21). Docker on `71ee15f` then **Verified**. Keep the Failed row in the [honesty ledger](honesty-ledger.md).
- **Live identity `.text` yank Failed** (unhandled sync on first `println!` — rustc `dyn Write` vtables are identity fn pointers). **Sensor / cut:** ADR-019 kept live `.text` and tore a 16 KiB dedicated range. ADR-020 then rewrote those vtables to high aliases and tore live `.text`. cts-ai Docker + GHA on `e80dc93` **Verified** that cut (`ident: reloc n=12`, live pages=37, 50 tests). Keep the Failed first yank.
- **PR #27 `ac3ad0b` GHA Failed** (`ident: range` then `ident: probe missed` — init still required a 4 KiB tear page). **Sensor:** 16 KiB range + relaxed init. Merge `24d94e6` GHA [34640667227](https://github.com/artofdream/ctos/actions/runs/34640667227) **Verified**. Keep the Failed SHA.

## No self-approval / no self-merge

The author does not `APPROVE` or merge their own PR. Same GitHub login is not a second identity. MRC writes `COMMENT` and names author / reviewer / merger. Merge hat is the *other* identity (`artofdream` vs `cursor[bot]`). See [ADR-002](../03-adr/ADR-002-pr-identity-split.md) and `.cursor/skills/ctos-mr-coordinator/SKILL.md`. Do not enable GitHub author self-APPROVE to make the gate “count.”
