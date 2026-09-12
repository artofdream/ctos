# KPIs / how we measure

Plain English. These are the probes this repo actually runs. They are **not** a product dashboard, a latency promise, or invented KPIs. Numbers live in the [honesty ledger](../framework/honesty-ledger.md) as one environment and one revision.

The three first-class pillars are antifragility ([NFR-05](../02-requirements/fr-nfr.md)), security ([NFR-10](../02-requirements/fr-nfr.md)), and performance ([NFR-07](../02-requirements/fr-nfr.md)). “Application support” below is an honest **scope** statement, not a new requirement ID.

## Performance

What we measure **today** on QEMU `virt` (serial markers + tests, fail-closed in `scripts/qemu-smoke.sh`).

**CNTPCT** is the CPU’s physical cycle counter — a hardware clock we read to see that time moved, not a published bench.

| Probe | What it is | What it is not |
| --- | --- | --- |
| Cycle-counter loop (`perf: cntpct`) | The counter is readable and moves over a fixed trivial loop | A microsecond budget, interrupt latency, or “faster than X” |
| IRQ-to-handler delta (`perf: irq-delta`) | Counter minus the timer’s compare value; min / max / spread on several ticks | A latency SLA, SPEC, or a comparison to other kernels |
| Boot-delta (`perf: boot-delta`) | Cycle count from after paging init to after `Hello World!` | QEMU process start time, a boot budget, or a published bench |
| Host ELF size (`perf: elf-size`) | Byte size of the debug `ctos` ELF after `cargo build` | A size budget or “smaller is better” |
| App-load (`perf: app-load`) | Cycle count around FAT `/hello` read + A3 map + `ERET` (A9) | A delta vs the embed, a budget, or “slots are free” |

QEMU `virt` is **one guest**. It is not Raspberry Pi, not real silicon, and not SPEC. Optimize only after a probe shows a cost. Details: [performance.md](../framework/performance.md).

### OS slot vs app slot (performance)

[ADR-030](../03-adr/ADR-030-os-app-slots.md) first cut exists: FAT `/hello` + `perf: app-load`. [ADR-031](../03-adr/ADR-031-track-a-leftovers.md) removes the A2–A4 embed. [Immutability](advantages.md#immutability) means disconnect OS update from apps. That can **cost** at runtime or be **neutral**. We do **not** invent a percentage, a budget, or “faster than linking the app into the kernel.” A Verified *delta* vs the old embed stays **Planned**.

| Kind | Honest guess | What would make it a claim |
| --- | --- | --- |
| Possible **cost** | Extra VFS/FAT read, ELF parse, user map fills, SVC, ASID/TTBR switch | Compared `perf: app-load` vs the embed on the same guest. Marker alone is not a delta. |
| Possible **neutral** | Steady-state UART print / yield after the app is mapped, if the hot path stays similar | Same probes on the new path vs the in-tree workers; no win claimed without a delta |
| Build-time, not a bench | Two host artifacts; FAT slot can change without a kernel rebuild | `perf: elf-size` is still one image’s byte count. A kernel rebuild still compiles the hello (`build.rs`); it no longer embeds the bytes. |

**Measure first** ([NFR-07](../02-requirements/fr-nfr.md)). Do not tune a loader “for speed” on a hunch. Do not copy QEMU ticks into a product slide.

## Stability / antifragility

We do not publish an uptime KPI. We measure whether **sensors stay fail-closed** and whether a repeated miss becomes a ratchet ([NFR-05](../02-requirements/fr-nfr.md), [antifragility.md](../framework/antifragility.md)).

| Mechanism | What it proves |
| --- | --- |
| `scripts/qemu-smoke.sh` | Build + required serial strings + tests exit 0. Missing a marker is a **fail**, not a skip. |
| `cargo test --features force-fail` | The panic path is fail-closed (host/QEMU exit 1). A green suite that cannot fail is not a sensor. |
| Honesty ledger | Every status word is a claim. **Verified** needs a probe. **Unknown** = unprobed. **Planned** = not built yet. **Failed** = the probe ran and lost. File presence is not QEMU boot. |
| Host ratchets | When a machine finds what CI missed, we keep the Failed row and add a sensor. Example: cts-ai Docker on `main` `b2bbb99` missed `paging: ok` after a larger nightly layout; GHA had been green. |

A green GitHub Actions run is one pair of runners. It is not “every host.” Docker on cts-ai is a separate row.

## Application support

Scope only — not a new requirement ID. Concrete examples and the cannot-run list: [What can run today](what-can-run.md).

Security work is probed (heap not-executable, stack guards, read-only code / not-executable data, user-mode miles) — that is **not** a product security KPI. See [security.md](../framework/security.md). Do not say “production ready.”
