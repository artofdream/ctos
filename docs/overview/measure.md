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
| App-load (`perf: app-load`) | Cycle count around FAT `/hello` read + A3 map + `ERET` (A9) | A budget, or “slots are free” |
| Embed-load (`perf: embed-load`) | Probe-only CNTPCT around map + `ERET` of the same linked-in hello ELF ([ADR-046](../03-adr/ADR-046-slot-perf-delta.md)) | Production slot path, `slot: embed`, or a bench |
| Slot delta (`perf: slot-delta`) | Raw `app=<a> embed=<b>` tick pair on one boot | A percent, “faster/slower,” or SPEC |
| FAT write (`perf: fat-write`) | CNTPCT around one FAT VFS write of the small `/probe` rewrite payload ([ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md)) | A write latency SLA or “disk cost” |
| Memfs write (`perf: memfs-write`) | CNTPCT around one memfs VFS write of the same bytes on the same boot | A memfs SLA or published bench |
| FS write delta (`perf: fs-write-delta`) | Raw `fat=<a> memfs=<b>` tick pair on one boot | A percent, “faster/slower,” or SPEC |
| FAT read (`perf: fat-read`) | CNTPCT around one FAT VFS read of `/probe` (`fat-hi`) ([ADR-065](../03-adr/ADR-065-fat-memfs-read-cntpct.md)) | A read latency SLA or “disk cost” |
| Memfs read (`perf: memfs-read`) | CNTPCT around one memfs VFS read of the same bytes on the same boot | A memfs SLA or published bench |
| FS read delta (`perf: fs-read-delta`) | Raw `fat=<a> memfs=<b>` tick pair on one boot | A percent, “faster/slower,” or SPEC |
| Net-ping (`perf: net-ping`) | CNTPCT around EL0 `net_ping` quiet ARP+ICMP vs SLIRP (`el0_icmp_ping`) ([ADR-069](../03-adr/ADR-069-net-ping-cntpct.md)) | A ping latency SLA, “has networking,” or SPEC |
| UDP DNS (`perf: udp-dns`) | CNTPCT around EL0 `net_udp_dns` quiet ARP+UDP DNS vs SLIRP (`el0_udp_dns`) ([ADR-072](../03-adr/ADR-072-udp-dns-cntpct.md)) | A DNS latency SLA, “has networking,” or SPEC |

QEMU `virt` is **one guest**. It is not Raspberry Pi, not real silicon, and not SPEC. Optimize only after a probe shows a cost. Details: [performance.md](../framework/performance.md).

### OS slot vs app slot (performance)

[ADR-030](../03-adr/ADR-030-os-app-slots.md) first cut exists: FAT `/hello` + `perf: app-load`. [ADR-032](../03-adr/ADR-032-track-a-leftovers.md) removes the A2–A4 production embed. [ADR-046](../03-adr/ADR-046-slot-perf-delta.md) adds a **probe-only** linked-in trip and `perf: slot-delta app=<a> embed=<b>` on the same boot. [Immutability](advantages.md#immutability) means disconnect OS update from apps. That can **cost** at runtime or be **neutral**. We do **not** invent a percentage, a budget, or “faster than linking the app into the kernel.” The pair is a QEMU TCG **lab measurement**.

| Kind | Honest claim shape | What it is not |
| --- | --- | --- |
| Compared **pair** | `perf: app-load` and `perf: embed-load` ticks plus `perf: slot-delta app=<a> embed=<b>` on one guest | A percent, SLA, or “slots are free” |
| Possible **cost** | Extra VFS/FAT read before the shared parse/map/`ERET` work | Marketing “slower than embed” without naming the tip + ticks |
| Possible **neutral** | Steady-state UART print / yield after the app is mapped | A win claimed without a probe |
| Build-time, not a bench | Two host artifacts; FAT slot can change without a kernel rebuild | `perf: elf-size` is still one image’s byte count. Kernel may `include_bytes!` only in `src/slot.rs` for the probe. |


### FAT write vs memfs write (performance)

[ADR-050](../03-adr/ADR-050-fat16-write.md) is the write mile. [ADR-051](../03-adr/ADR-051-fat-memfs-write-cntpct.md) adds a same-boot CNTPCT pair: `perf: fat-write` vs `perf: memfs-write` plus `perf: fs-write-delta fat=<a> memfs=<b>`. That can show a **cost** shape for going through virtio-blk/FAT versus in-RAM memfs — or be noisy under TCG. We do **not** invent a percentage, a budget, or “FAT write is X× slower.”

| Kind | Honest claim shape | What it is not |
| --- | --- | --- |
| Compared **pair** | Raw tick counts for the same small payload on one guest | A percent, SLA, or product KPI |
| Possible **cost** | Extra block/FAT work vs memfs memcpy | Marketing “slower” without naming the tip + ticks |
| Gate | Keep `fat: write` / `fat: ok`; fail closed on missing pair markers | Criterion / invented benches |

**Measure first** ([NFR-07](../02-requirements/fr-nfr.md)). Do not tune a loader “for speed” on a hunch. Do not copy QEMU ticks into a product slide.

### FAT read vs memfs read (performance)

[ADR-065](../03-adr/ADR-065-fat-memfs-read-cntpct.md) adds the symmetric same-boot CNTPCT pair for **read**: `perf: fat-read` vs `perf: memfs-read` plus `perf: fs-read-delta fat=<a> memfs=<b>`. Same honesty rules as the write pair — measurement only, not a latency SLA.

| Kind | Honest claim shape | What it is not |
| --- | --- | --- |
| Compared **pair** | Raw tick counts for the same small `/probe` payload on one guest | A percent, SLA, or product KPI |
| Possible **cost** | Extra block/FAT work vs memfs memcpy | Marketing “slower” without naming the tip + ticks |
| Gate | Keep `fat: read` / `fat: ok` / ADR-051 write markers; fail closed on missing pair markers | Criterion / invented benches |


### Net-ping path (performance)

[ADR-069](../03-adr/ADR-069-net-ping-cntpct.md) times the EL0 `net_ping` quiet path: `perf: net-ping ticks=<n>`. Smoke checks the **prefix** only — tick values vary under QEMU TCG. Optional N2 crumbs `perf: net-icmp-tx` / `perf: net-icmp-rx` remain lab-only ([ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md)).

| Kind | Honest claim shape | What it is not |
| --- | --- | --- |
| Single-path **sample** | Raw tick count for quiet ARP+ICMP as `SYS_NET_PING` does it | A percent, SLA, or product KPI |
| Gate | Prefix `perf: net-ping` present; reject faster/slower/percent | Exact tick count; criterion / invented benches |
| Non-claim | — | TCP/UDP, sockets, “has networking,” Wi‑Fi |

### UDP DNS path (performance)

[ADR-072](../03-adr/ADR-072-udp-dns-cntpct.md) times the EL0 `net_udp_dns` quiet path: `perf: udp-dns ticks=<n>`. Smoke checks the **prefix** only — tick values vary under QEMU TCG. Window = quiet ARP + UDP DNS as `SYS_NET_UDP_DNS` / `el0_udp_dns` (same pattern as ADR-069).

| Kind | Honest claim shape | What it is not |
| --- | --- | --- |
| Single-path **sample** | Raw tick count for quiet ARP+UDP DNS as `SYS_NET_UDP_DNS` does it | A percent, SLA, or product KPI |
| Gate | Prefix `perf: udp-dns` present; reject faster/slower/percent | Exact tick count; criterion / invented benches |
| Non-claim | — | TCP, sockets, DNS product, “has networking,” Wi‑Fi |

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
