# ADR-011 — Three pillars: antifragility, security, performance

- Status: Accepted
- Date: 2026-09-09

Frozen [FR/NFR](../02-requirements/fr-nfr.md) **IDs** are unchanged. This ADR revises **NFR-05**, **NFR-07**, and **NFR-10** text in place. Do not mint FR-16+ or NFR-15+.

## Context

Bring-up M0–M9 is on `main` (UART through a cooperative EL1 scheduler). The first-class practice so far was the honesty/harness loop ([ADR-001](ADR-001-honesty-harness-for-ctos.md)): probes, ratchets, fail-closed CI, no self-merge.

Sponsor direction after M9: **antifragility is not enough by itself**. Security and performance are first-class pillars too — not afterthoughts bolted on when the kernel “feels done.”

Risks if we leave the NFR text as-is:

- **NFR-07** is still “Could / Later / learning-only.” That invites fake benches or silent “it’s fast enough” claims.
- **NFR-10** is mostly “no secrets in repo.” That is necessary and not a threat model.
- **NFR-05** already names ratchets, but it reads like a Next-stage SOP rather than a standing pillar.

This ADR does **not** clone florist / AEA vocabulary. The three pillars are ctos-native names for existing IDs.

## Decision

1. **Three first-class pillars**, same weight, different sensors:
   - **Antifragility** — keep the existing harness: honesty ledger, fail-closed `scripts/qemu-smoke.sh`, GHA `smoke.yml`, Docker smoke, `#[test_case]`. Repeated failures become ratchets, not README-only advice ([antifragility.md](../framework/antifragility.md)).
   - **Security** — no “secure OS” / “hardened” / “W^X done” claim without a written threat model **and** a probe. Prefer minimize `unsafe` (already NFR-01). Prefer W^X / NX stacks and heap when paging can express it. No execute-from-writable heap **by default** once maps can mark NX. Least privilege on IRQ paths (no heap alloc, no `sched::yield_now` from an IRQ). Future EL0 isolation is **Planned**.
   - **Performance** — no fake benches and no invented latency numbers. Add honest measurable probes (CNTPCT delta around a known path; later timer-tick jitter). Optimize only after a probe shows a cost.

2. **Revise frozen NFR text in place** (IDs stay NFR-05 / NFR-07 / NFR-10):
   - NFR-05: Must / Now. Ratchets stay the mechanism.
   - NFR-07: Should / Now. Probe language; not a hard latency budget.
   - NFR-10: Must / Now. Threat-model stub + claim gate, not only “no secrets.”

3. **W^X is Planned on this identity map.** [ADR-008](ADR-008-identity-map-frame-allocator.md) maps virt RAM as one executable 1 GiB L1 Normal block. The heap ([ADR-009](ADR-009-first-fit-heap.md)) and cooperative stacks ([ADR-010](ADR-010-cooperative-rr-el1.md)) live in that block, so they are W+X today. Forbidding execute-from-heap needs an L2/L3 split (or a later higher-half map). Device MMIO is already XN. Do not claim heap NX because the L3 map window can set PXN — that window is not the heap.

4. **One small code ratchet may land with this docs PR.** This change lands a baseline `CNTPCT` loop delta (serial `perf: cntpct` + `#[test_case]`). It does **not** split the L1 RAM block to fake W^X. Honesty over heroics.

5. **Post-M9 work is a pillars section**, not a second bring-up stack on an open PR. Threat-model v1, W^X / NX heap+stacks, and further perf probes were listed as later loop units. The sponsor later asked for one coherent follow-up ([ADR-012](ADR-012-wx-nx-heap-stacks.md), [ADR-013](ADR-013-el0-isolation-direction.md), irq-delta) rather than conflicting parallel branches.

## Consequences

- README, `AGENTS.md`, the roadmap, and the honesty ledger point at [pillars.md](../framework/pillars.md).
- A PR that says “secure” or “faster” without a ledger row is a coherence fail (NFR-06 / NFR-13).
- The CNTPCT probe (when landed) is a **baseline that the counter advances**. It is not a published benchmark and not a comparison to other kernels.
- Standing EL0, PAN, ASID-tagged TLB isolation, and preemption remain later. RO+NX text/data is [ADR-015](ADR-015-ro-nx-text-data.md). Stack guard pages are [ADR-014](ADR-014-linker-stack-guard-pages.md). EL0 first mile + user-TTBR0 read mile are [ADR-013](ADR-013-el0-isolation-direction.md) (isolation still Planned).
