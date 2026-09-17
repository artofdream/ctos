# ADR-078 — Thin TCP echo path CNTPCT (measure-first)

- Status: Accepted (lab CNTPCT on QEMU TCG — not a published bench).
- Date: 2026-09-17

## Context

[ADR-069](ADR-069-net-ping-cntpct.md) timed EL0 `net_ping`. [ADR-072](ADR-072-udp-dns-cntpct.md) timed EL0 `net_udp_dns`. [ADR-074](ADR-074-n5-thin-tcp.md) closed **N5** kernel thin TCP (`net: tcp-ok`). [ADR-076](ADR-076-el0-tcp-svc.md) closed **N5.x** with EL0 `SYS_NET_TCP_ECHO` (28) and freestanding `/tcpdemo`, and **deferred** TCP CNTPCT.

Perf slice next asks for another **measure-first** probe around a known path. The natural cut is the **EL0 `net_tcp_echo` quiet path** (kernel-owned ARP + thin TCP active-open + one payload vs QEMU `guestfwd` `10.0.2.4:7`) that ADR-076 already exercises — same pattern as ADR-069 / ADR-072.

Do **not** invent a latency SLA, percent, or “faster/slower” claim. Do **not** add a `criterion` crate. Do **not** implement BSD sockets, listen/accept, TLS, or widen the EL0 net ABI. Keep N1–N5 / N5.x / N3.x / N4 markers, ADR-069/072 prefixes, FAT nested/rmdir, and all prior smoke greps. Force-fail timeout stays **180s**.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Sample `CNTPCT` around the full quiet `el0_tcp_echo` body (ARP + TCP SYN/est/tx/rx as `SYS_NET_TCP_ECHO` / `/tcpdemo` does it). Print `perf: tcp-echo ticks=<n>`. Fail closed if the counter does not advance after a successful probe. Smoke greps the **prefix** `perf: tcp-echo` (not an exact tick count). | Honest single-path measurement. Values vary under TCG. Matches ADR-069/072. **Chosen surface: EL0 `net_tcp_echo`** (not a separate kernel-only N5 timer). |
| **H2 (rejected)** | Publish a percent / “TCP is X µs” / criterion bench / SLA. | NFR-07 forbids invented benches. |
| **H3 (rejected)** | Only time the kernel N5 `net: tcp-*` path and rename it `tcp-echo`. | Kernel N5 markers stay as ADR-074 transport proof; ADR-078 names the EL0 SVC round-trip (same choice as ADR-069/072 H3). |
| **H4 (rejected)** | Require an exact tick count in smoke. | Flaky under QEMU TCG jitter. |
| **H5 (rejected)** | Add listen/accept / sockets / TLS or widen the EL0 net ABI this mile. | Out of scope; honesty fail. |

## Decision

1. **Window.** In `virtio::el0_tcp_echo` (called from `SYS_NET_TCP_ECHO` / FAT `/tcpdemo`), sample `CNTPCT` from before the quiet ARP post through a matching TCP echo payload. That is the EL0 `net_tcp_echo` round-trip the sample already runs — not “TCP latency” as a product KPI. **Chosen surface: EL0 `net_tcp_echo`** (document explicitly; kernel `net: tcp-ok` path unchanged / untimed this mile).
2. **Marker (exact form).** On success with a non-zero delta: `perf: tcp-echo ticks=<n>` (decimal ticks, no units, no percent). If the probe succeeds but the counter did not advance: `perf: tcp-echo missed` and treat the probe as failed.
3. **Smoke.** `scripts/qemu-smoke.sh` greps for the prefix `perf: tcp-echo` (presence). Rejects lines that claim faster/slower/percent. Does **not** assert a numeric tick value. Keep all prior greps (blk/fat/slot/net/ping/udp/tcp/samples/ADR-069/072/076/077). Default `CTOS_FORCE_FAIL_TIMEOUT` remains **180s**.
4. **Test.** `#[test_case] el0_tcp_echo_cntpct_advances` asserts a successful quiet TCP echo and `tcp_echo_ticks() > 0`.
5. **Honesty.** QEMU `virt` TCG lab only. Not a latency budget, not SPEC, not “has networking,” not sockets. Record Verified only with serial evidence on a named tip. CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: `src/virtio.rs` (`el0_tcp_echo` CNTPCT + `perf: tcp-echo` + `tcp_echo_ticks`); smoke greps; `#[test_case]`.
- Docs: this ADR, [performance.md](../framework/performance.md), [measure.md](../overview/measure.md), honesty ledger, roadmap `P-PERF-10`, SUMMARY, threat-model light bump (**v1.53**), NFR-10 note.
- Follow-ups (not this PR): richer socket ABI, listen-accept, TLS — new ADR + sponsor only.
