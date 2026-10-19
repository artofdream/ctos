# ADR-106 — Proposed multi-core claim sentence (docs only; not accepted)

- Status: **Proposed, not accepted; needs the sponsor's written acceptance.**
- Date: 2026-10-19
- Follows: [ADR-091](ADR-091-el0-isolated-accept-draft.md) rev 4 (the current accepted claim), [ADR-100](ADR-100-smp-race-story-plan.md)–[ADR-105](ADR-105-smp-s5-concurrent-el0-runq.md) (the SMP race story, opt-in probes only).
- This PR changes **no code** and **does not** edit the ADR-091 rev 4 claim. Until the sponsor writes an accept, the accepted claim stays one core.

## Proposed sentence

> On QEMU TCG, `-cpu cortex-a76`, `-smp 2`, the opt-in `smp2-probe` build has demonstrated: (1) each core has its own `TPIDR_EL1` → PerCpu block, exception / thread / fatal stacks and expected-fault flags; (2) broadcast TLBI (`tlbi …is`) removes a mapping from the other core's TLB; (3) an ASID allocator with generations keeps a running core's ASID reserved across a rollover (`tlbi vmalle1is`), and dropping that rule is caught; (4) two tasks with private TTBR0 tables and allocator ASIDs run EL0 concurrently on both cores, with a reach / task-to-task / execute-only check over both spaces. This is **not** a claim that "EL0 is isolated" on two cores in the ADR-091 sense: the default build is still one core; the probes are opt-in; TCG flushes a vCPU's soft TLB on any ASID change (so ASID tagging itself is not shown); there is no preemptive cross-core scheduler; and no KVM or real-hardware evidence is attached.

## Why this shape

- Tight to what ADR-102..105 actually printed on the box and in CI.
- Keeps ADR-091 rev 4 as the accepted product claim until a written accept replaces or extends it.
- Names the TCG / ASID-tagging limit explicitly so a future reader cannot treat the rollover proof as a hardware ASID-tag proof.

## Accept path

A written sponsor accept of this sentence (or a tightened revision of it). On accept: move this ADR to Accepted, add a one-line pointer from ADR-091 (without rewriting rev 4's one-core sentence unless the accept says to), and add a ledger row. No code change is required for the accept itself.
