# Three pillars

After M9, ctos treats **antifragility**, **security**, and **performance** as first-class pillars ([ADR-011](../03-adr/ADR-011-three-pillars.md)). They share one rule: a status word needs a probe ([honesty ledger](honesty-ledger.md)).

| Pillar | Frozen ID | Home | What “done” looks like |
| --- | --- | --- | --- |
| Antifragility | [NFR-05](../02-requirements/fr-nfr.md) | [antifragility.md](antifragility.md) | Repeated failures become sensors/gates, not extra README advice |
| Security | [NFR-10](../02-requirements/fr-nfr.md) | [security.md](security.md) | Threat-model v1.5 + probes before any “secure OS” claim. EL0 miles: [el0.md](el0.md) (umbrella isolation Planned; standing + TTBR1 first cut + EL1 high-VA fetch + ASID are separate rows) |
| Performance | [NFR-07](../02-requirements/fr-nfr.md) | [performance.md](performance.md) | Measurable CNTPCT + IRQ-delta + host ELF size + boot-delta; optimize only with a probe |

Bring-up M0–M9 stays on the [roadmap](../04-roadmap/roadmap.md). Pillar work after M9 is listed there as a separate section so a docs PR does not pretend to close paging or a scheduler.

Do not import florist / AEA role names. These pillars are ctos-native.
