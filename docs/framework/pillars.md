# Three pillars

After the cooperative scheduler (M9), ctos treats **antifragility**, **security**, and **performance** as first-class pillars ([ADR-011](../03-adr/ADR-011-three-pillars.md)). In everyday words: we turn repeated misses into sensors, we prove security slices instead of saying “secure OS,” and we measure before we tune. They share one rule: a status word needs a probe ([honesty ledger](honesty-ledger.md)).

**Principles drive; pillars sit under them; tracks sit under both.** Visitor-facing list: [landing — Driving principles](../index.md#driving-principles). This page is the deep hub, not a second marketing copy. Extra stance: [overview.md](overview.md), [apps-today.md](apps-today.md), [building-or-porting.md](building-or-porting.md), [immutability.md](immutability.md).

```mermaid
flowchart TD
  P["Core principles<br/>honesty · antifragility · security<br/>performance · document-first"]
  L["Three pillars — this page<br/>NFR-05 · NFR-10 · NFR-07"]
  T["Tracks A / B<br/>subordinate workstreams"]
  P --> L --> T
```

*Tracks (loader/ABI, later slots/FS) do not outrank a principle.*

| Pillar | Frozen ID | Home | What “done” looks like |
| --- | --- | --- | --- |
| Antifragility | [NFR-05](../02-requirements/fr-nfr.md) | [antifragility.md](antifragility.md) | Repeated failures become sensors/gates, not extra README advice |
| Security | [NFR-10](../02-requirements/fr-nfr.md) | [security.md](security.md) | Threat-model v1.8 + probes before any “secure OS” claim. EL0 miles: [el0.md](el0.md). On `main` `e80dc93`: standing + ASID + TTBR1 first cut + EL1 high-VA fetch + identity-tear first cut + 16 KiB range + live `.text` tear (ADR-020) are Verified. Still **Planned**: `.rodata`/`.data`/heap tear, PAN on `cortex-a57`, umbrella isolation. |
| Performance | [NFR-07](../02-requirements/fr-nfr.md) | [performance.md](performance.md) | Measurable CNTPCT + IRQ-delta + host ELF size + boot-delta; optimize only with a probe |

Bring-up M0–M9 stays on the [roadmap](../04-roadmap/roadmap.md). Pillar work after M9 is listed there as a separate section so a docs PR does not pretend to close paging or a scheduler.

Immutability is the same rule: **scoped** RO+NX / text-tear probes are fine; **absolute** “immutable OS” is not. Overview: [Advantages — Immutability](../overview/advantages.md#immutability).

Do not import florist / AEA role names. These pillars are ctos-native.
