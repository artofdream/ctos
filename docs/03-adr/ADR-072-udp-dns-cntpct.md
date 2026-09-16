# ADR-072 — UDP DNS path CNTPCT (measure-first)

- Status: Accepted (lab CNTPCT on QEMU TCG — not a published bench).
- Date: 2026-09-16

## Context

[ADR-069](ADR-069-net-ping-cntpct.md) landed CNTPCT around the EL0 `net_ping` quiet path. [ADR-070](ADR-070-n3-udp-transport.md) closed **N3** kernel UDP (`net: udp-ok`). [ADR-071](ADR-071-n3x-el0-udp-svc.md) closed **N3.x** with EL0 `SYS_NET_UDP_DNS` (26) and freestanding `/udpdemo`.

Perf slice next asks for another **measure-first** probe around a known path. The natural cut is the **EL0 `net_udp_dns` quiet path** (kernel-owned ARP + UDP DNS query vs SLIRP `10.0.2.3:53`) that ADR-071 already exercises — same pattern as ADR-069 on `net_ping`.

Do **not** invent a latency SLA, percent, or “faster/slower” claim. Do **not** add a `criterion` crate. Do **not** implement FAT mkdir or TCP. Keep N1/N2/N3/N3.x/N4 markers and all prior smoke greps (including ADR-069 `perf: net-ping`).

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Sample `CNTPCT` around the full quiet `el0_udp_dns` body (ARP + UDP DNS RTT as `SYS_NET_UDP_DNS` / `/udpdemo` does it). Print `perf: udp-dns ticks=<n>`. Fail closed if the counter does not advance after a successful probe. Smoke greps the **prefix** `perf: udp-dns` (not an exact tick count). | Honest single-path measurement. Values vary under TCG. Matches ADR-069. |
| **H2 (rejected)** | Publish a percent / “DNS is X µs” / criterion bench / SLA. | NFR-07 forbids invented benches. |
| **H3 (rejected)** | Only time the kernel N3 `net: udp-tx`/`udp-rx` path and rename it `udp-dns`. | Kernel N3 markers stay as ADR-070 transport proof; ADR-072 names the EL0 SVC round-trip (same choice as ADR-069 H3). |
| **H4 (rejected)** | Require an exact tick count in smoke. | Flaky under QEMU TCG jitter. |
| **H5 (rejected)** | Add TCP / FAT mkdir / sockets or widen the EL0 net ABI this mile. | Out of scope; sponsor mile (2) is FAT mkdir — not this PR. |

## Decision

1. **Window.** In `virtio::el0_udp_dns` (called from `SYS_NET_UDP_DNS` / FAT `/udpdemo`), sample `CNTPCT` from before the quiet ARP post through a matching UDP DNS reply. That is the EL0 `net_udp_dns` round-trip the sample already runs — not “DNS latency” as a product KPI. **Chosen surface: EL0 `net_udp_dns`** (not a separate kernel-only probe).
2. **Marker (exact form).** On success with a non-zero delta: `perf: udp-dns ticks=<n>` (decimal ticks, no units, no percent). If the probe succeeds but the counter did not advance: `perf: udp-dns missed` and treat the probe as failed.
3. **Smoke.** `scripts/qemu-smoke.sh` greps for the prefix `perf: udp-dns` (presence). Rejects lines that claim faster/slower/percent. Does **not** assert a numeric tick value. Keep all prior greps (blk/fat/slot/net/ping/udp/samples/ADR-069/ADR-071).
4. **Test.** `#[test_case] el0_udp_dns_cntpct_advances` asserts a successful quiet UDP DNS probe and `udp_dns_ticks() > 0`.
5. **Honesty.** QEMU `virt` TCG lab only. Not a latency budget, not SPEC, not “has networking,” not sockets, not a DNS product. Record Verified only with serial evidence on a named tip. CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: `src/virtio.rs` (`el0_udp_dns` CNTPCT + `perf: udp-dns` + `udp_dns_ticks`); smoke greps; `#[test_case]`.
- Docs: this ADR, [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, roadmap `P-PERF-9`, SUMMARY, threat-model light bump (**v1.47**).
- Follow-ups (not this PR): FAT mkdir (sponsor mile 2), TCP — new ADR + sponsor only.
