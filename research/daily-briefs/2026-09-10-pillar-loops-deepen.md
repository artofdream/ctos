# Daily brief — 2026-09-10 (pillar loops 2–6)

## Where we stopped

Draft PR https://github.com/artofdream/ctos/pull/19 (`cursor/pillar-loops-deepen-48ae`). Parent is `main` `1eb82b8` (Merge PR #18). One PR: threat-model v1.1, ADR-014 stack guards, host ELF size, EL0 first mile, Obsidian polish. No new FR/NFR IDs.

Cloud `scripts/qemu-smoke.sh` **Verified** (2026-09-10, QEMU 8.2.2, `rustc` 1.100.0-nightly `a36d05efa`): host `perf: elf-size bytes=3752056`; `Hello World!`, `paging: ok`, `heap: ok`, `sched: ok`, `wx: nx heap` / `wx: ok`, `guard: fault` / `guard: ok`, `el0: svc` / `el0: nx kernel` / `el0: ok`, `perf: cntpct delta=20592`, `timer: tick`, `perf: irq-delta min=14477 max=24149 spread=9672 n=8`, `input: rx 0x41`, two `exception: sync BRK`, `exception: fatal nested` / `kind=0x200`, `Running 29 tests` all `[ok]`, force-fail exit 1.

GHA `smoke.yml` on this branch: **Unknown** (no run URL yet).

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #19 is `cursor[bot]`; merge hat is `artofdream`.
2. Isolation (separate map / PAN / ASID) is still Planned. Do not claim “EL0 isolated.”
3. Linker stack **pages** stay executable. Do not claim “the kernel is W^X.”

## Honesty

- Threat-model v1.1 is a file+review Verified. “Secure OS” unclaimed.
- Guard pages and EL0 first mile Verified on this QEMU virt guest only.
- Host ELF size is a measurement, not a budget.
- cts-ai Docker Desktop path not re-run.
- `.obsidian/` was not committed.
