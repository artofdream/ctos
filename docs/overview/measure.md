# KPIs / how we measure

Plain English. These are the probes this repo actually runs. They are **not** a product dashboard, a latency SLA, or invented KPIs. Numbers live in the [honesty ledger](../framework/honesty-ledger.md) as one environment and one revision. Deep dives: [pillars](../framework/pillars.md), [performance](../framework/performance.md), [antifragility](../framework/antifragility.md).

The three first-class pillars are antifragility ([NFR-05](../02-requirements/fr-nfr.md)), security ([NFR-10](../02-requirements/fr-nfr.md)), and performance ([NFR-07](../02-requirements/fr-nfr.md)). “Application support” below is an honest **scope** statement, not a new FR/NFR ID.

## Performance

What we measure **today** on QEMU `virt` (serial markers + `#[test_case]`, fail-closed in `scripts/qemu-smoke.sh`):

| Probe | What it is | What it is not |
| --- | --- | --- |
| CNTPCT loop (`perf: cntpct`) | The physical counter is readable and moves over a fixed trivial loop | A microsecond budget, interrupt latency, or “faster than X” |
| IRQ-to-handler delta (`perf: irq-delta`) | `CNTPCT − CNTP_CVAL` min / max / spread on several timer ticks | A latency SLA, SPEC, or a comparison to other kernels |
| Boot-delta (`perf: boot-delta`) | CNTPCT from after `paging::init` to after `Hello World!` | QEMU process start time, a boot budget, or a published bench |
| Host ELF size (`perf: elf-size`) | Byte size of the debug `ctos` ELF after `cargo build` | A size budget or “smaller is better” |

QEMU `virt` / TCG (or the host’s QEMU) is **one guest**. It is not Raspberry Pi, not real silicon, and not SPEC. Optimize only after a probe shows a cost. Details: [performance.md](../framework/performance.md).

### OS slot vs app slot (performance)

Direction only — Track A is **not built**. [Immutability](advantages.md#immutability) means disconnect OS update from apps. That can **cost** at runtime or be **neutral**. We do **not** invent a percentage, a budget, or “faster than linking the app into the kernel.”

| Kind | Honest guess (unmeasured) | What would make it a claim |
| --- | --- | --- |
| Possible **cost** | Extra `ERET` / `SVC`, a TTBR0 switch, mapping an app slot — vs today’s in-tree function call | CNTPCT (or irq-delta) around a real load + enter/leave once Track A exists |
| Possible **neutral** | Steady-state UART print / yield after the app is mapped, if the hot path stays similar | Same probes on the new path vs the in-tree workers; no win claimed without a delta |
| Build-time, not a bench | Kernel ELF no longer contains the “app”; you rebuild slots separately | `perf: elf-size` is still one image’s byte count, not “smaller is better” |

**Measure first** ([NFR-07](../02-requirements/fr-nfr.md)). Do not tune the loader “for speed” on a hunch. Do not copy QEMU TCG ticks into a product slide.

## Stability / antifragility

We do not publish an uptime KPI. We measure whether **sensors stay fail-closed** and whether a repeated miss becomes a ratchet ([NFR-05](../02-requirements/fr-nfr.md), [antifragility.md](../framework/antifragility.md)).

| Mechanism | What it proves |
| --- | --- |
| `scripts/qemu-smoke.sh` | Build + required serial strings + `cargo test` exit 0. Missing a marker is a **fail**, not a skip. |
| `cargo test --features force-fail` | The panic path is fail-closed (host/QEMU exit 1). A green suite that cannot fail is not a sensor. |
| Honesty ledger | Every status word is a claim. **Verified** needs a probe. **Unknown** = unprobed. **Planned** = not built yet. **Failed** = the probe ran and lost. File presence is not QEMU boot. |
| Host ratchets | When a machine finds what CI missed, we keep the Failed row and add a sensor. Example: cts-ai Docker on `main` `b2bbb99` missed `paging: ok` after a larger nightly layout; GHA had been green. The layout L3 / post-MMU frame-init ratchet is in the [ledger](../framework/honesty-ledger.md). |

A green GitHub Actions run is one pair of runners. It is not “every host.” Docker on cts-ai is a separate row.

## Application support

Scope only — not a new FR/NFR ID. Concrete examples and the cannot-run list: [What can run today](what-can-run.md).

Security work is probed (heap NX, stack guards, RO+NX, EL0 miles) — that is **not** a product security KPI. See [security.md](../framework/security.md). Do not say “production ready.”
