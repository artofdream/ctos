# ADR-069 — Net-ping path CNTPCT (measure-first)

- Status: Accepted (lab CNTPCT on QEMU TCG — not a published bench).
- Date: 2026-09-16

## Context

[ADR-065](ADR-065-fat-memfs-read-cntpct.md) landed a same-boot FAT vs memfs **read** CNTPCT pair. [ADR-067](ADR-067-virtio-net-icmp-ping.md) closed **N2** kernel-path ICMP (`net: ping-ok`) with optional crumbs `perf: net-icmp-tx` / `perf: net-icmp-rx`. [ADR-068](ADR-068-el0-net-svc-sample.md) closed **N4** with EL0 `SYS_NET_PING` (25) and freestanding `/netdemo`.

Perf slice next asks for another **measure-first** probe around a known path. The natural cut is the **EL0 `net_ping` quiet path** (kernel-owned ARP + ICMP echo vs SLIRP `10.0.2.2`) that ADR-068 already exercises.

Do **not** invent a latency SLA, percent, or “faster/slower” claim. Do **not** add a `criterion` crate. Do **not** unlock N3 sockets / TCP/UDP. Keep N1/N2/N4 markers and all prior smoke greps.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Sample `CNTPCT` around the full quiet `el0_icmp_ping` body (ARP + ICMP echo RTT as `SYS_NET_PING` does it). Print `perf: net-ping ticks=<n>`. Fail closed if the counter does not advance after a successful ping. Smoke greps the **prefix** `perf: net-ping` (not an exact tick count). | Honest single-path measurement. Values vary under TCG. |
| **H2 (rejected)** | Publish a percent / “ping is X µs” / criterion bench / SLA. | NFR-07 forbids invented benches. |
| **H3 (rejected)** | Only time the kernel N2 icmp-tx or icmp-rx crumb and rename it `net-ping`. | Those optional crumbs stay as ADR-067 lab notes; ADR-069 names the EL0 SVC round-trip. |
| **H4 (rejected)** | Require an exact tick count in smoke. | Flaky under QEMU TCG jitter. |
| **H5 (rejected)** | Add TCP/UDP/sockets or widen the EL0 net ABI this mile. | N3 locked ([ADR-063](ADR-063-network-foundation-scope.md)). |

## Decision

1. **Window.** In `virtio::el0_icmp_ping` (called from `SYS_NET_PING`), sample `CNTPCT` from before the quiet ARP post through a matching ICMP echo reply. That is the EL0 `net_ping` round-trip the sample already runs — not “network latency” as a product KPI.
2. **Marker (exact form).** On success with a non-zero delta: `perf: net-ping ticks=<n>` (decimal ticks, no units, no percent). If the ping succeeds but the counter did not advance: `perf: net-ping missed` and treat the probe as failed. Related existing optional crumbs (unchanged): `perf: net-icmp-tx ticks=<n>` / `perf: net-icmp-rx ticks=<n>` on the kernel N2 probe path.
3. **Smoke.** `scripts/qemu-smoke.sh` greps for the prefix `perf: net-ping` (presence). Rejects lines that claim faster/slower/percent. Does **not** assert a numeric tick value. Keep all prior greps (blk/fat/slot/net/ping/samples/ADR-065 pair).
4. **Test.** `#[test_case] el0_net_ping_cntpct_advances` asserts a successful quiet ping and `net_ping_ticks() > 0`.
5. **Honesty.** QEMU `virt` TCG lab only. Not a latency budget, not SPEC, not “has networking,” not sockets. Record Verified only with serial evidence on a named tip. CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: `src/virtio.rs` (`el0_icmp_ping` CNTPCT + `perf: net-ping` + `net_ping_ticks`); smoke greps; `#[test_case]`.
- Docs: this ADR, [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, roadmap `P-PERF-8`, SUMMARY, threat-model light bump (**v1.44**).
- Follow-ups (not this PR): yield/SVC CNTPCT pair, memcpy baseline — only after a probe shows a need. N3 sockets stay locked.
