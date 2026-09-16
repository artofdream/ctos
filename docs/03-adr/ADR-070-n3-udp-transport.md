# ADR-070 — Track N / N3 — UDP transport unlock (minimal IPv4 datagram)

- Status: Accepted (N3 runtime — **Verified** only when this tip’s `qemu-smoke` prints `net: udp-ok`; else Documented/partial)
- Date: 2026-09-16

## Context

[ADR-063](ADR-063-network-foundation-scope.md) opened **Track N** and left **N3 (transport / sockets) locked** until a new sponsor scope ADR. N1 ([ADR-066](ADR-066-virtio-net-first-frame.md)) Verified ARP first-frame (`net: ok`). N2 ([ADR-067](ADR-067-virtio-net-icmp-ping.md)) Verified ICMP echo (`net: ping-ok`). N4 ([ADR-068](ADR-068-el0-net-svc-sample.md)) Verified tiny EL0 `net_mac` / `net_ping` + `/netdemo`. Perf slice [ADR-069](ADR-069-net-ping-cntpct.md) measures EL0 `net_ping` CNTPCT.

Sponsor unlocked N3: proceed with a **real transport mile**, not docs-only forever. Prefer **UDP datagram first** (measurable, fail-closed on SLIRP) over a fake POSIX sockets layer. Do **not** ship a full BSD sockets / TCP product this PR.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | After N1 ARP + N2 ICMP on the same virtio-net path: TX one IPv4 **UDP** datagram to QEMU SLIRP DNS `10.0.2.3:53` (minimal DNS *query* as payload), RX any matching DNS *response* (QR=1, same TXID). Markers `net: udp-tx` / `net: udp-rx` / `net: udp-ok`. QEMU args **unchanged**. | Deterministic on GHA/agent-box (SLIRP answers UDP DNS locally). Proves L4 datagram without sockets/TCP. DNS here is **probe bait only**, not a DNS product. |
| **H2 (rejected)** | QEMU `guestfwd` UDP echo this mile. | Extra host plumbing; H1 is enough if green. Revisit only if SLIRP DNS proves flaky. |
| **H3 (rejected)** | Ship a BSD sockets ABI / TCP stack / “has networking / sockets OS” marketing. | Honesty fail; locked product non-goals. |
| **H4 (rejected)** | Docs-only unlock with no guest probe. | Sponsor asked for a real transport mile. |
| **H5 (rejected)** | Require EL0 UDP SVCs this mile. | Optional later (N3.x); kernel-path probe matches N2 style. Keep existing `net_mac`/`net_ping`. |

## Decision

1. **Unlock N3** under this ADR. Ladder status moves from Locked → **Verified** when this tip’s `qemu-smoke` prints `net: udp-ok` (with N1/N2 markers preserved).

2. **In scope this mile.**
   - Minimal **UDP TX/RX (IPv4)** on existing virtio-net (reuse ARP/IP helpers from the ICMP path).
   - Honest CI probe: UDP DNS query from guest `10.0.2.15` to SLIRP DNS `10.0.2.3:53`; accept any DNS response with matching transaction id and QR=1 (any RCODE).
   - Serial markers + fail-closed smoke greps.
   - Optional thin EL0 UDP SVC(s): **not** this PR (natural follow-up only with another ADR / N3.x).

3. **Still not product (locked).**
   - Full BSD sockets API.
   - TCP stack as product.
   - DHCP / DNS as product features (the probe may *use* SLIRP DNS as a peer; that is not a guest DNS resolver product).
   - Wi‑Fi; Linux net stack; marketing “has networking / sockets OS.”

4. **Packet.** Ethernet (dst = gateway MAC from N1 ARP sha) + IPv4 (src `10.0.2.15`, dst `10.0.2.3`, proto UDP, TTL 64) + UDP (src ephemeral `0x6333`, dst 53, checksum 0 allowed for IPv4) + minimal DNS A query for label `ctos` (TXID `0x6333`). No IP options. No TCP. No sockets ABI.

5. **Serial markers (N3).**
   - `net: udp-tx` — TX used ring advanced for the UDP datagram
   - `net: udp-rx` — RX delivered a matching UDP DNS response
   - `net: udp-ok` — N3 closed
   - Keep N1 (`net: ok`) and N2 (`net: ping-ok`) markers
   - Miss → `net: probe missed` (fail-closed)

6. **Smoke.** `scripts/qemu-smoke.sh` greps N1 + N2 **and** `net: udp-tx` / `net: udp-rx` / `net: udp-ok`. Must keep `blk:*` / `fat:*` / slot / sample / perf greps green.

7. **Future N3.x.** TCP and/or richer socket ABI only with **another ADR** + sponsor. Do not silently grow into a sockets product.

8. **Honesty.** Say “the guest sent one UDP datagram and received a UDP reply via SLIRP” only when serial/`qemu-smoke` print `net: udp-ok` on a named tip. Do not say the OS “has networking,” “has sockets,” or “has DNS.” CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: extend `src/virtio.rs` (`fill_udp_dns_query`, UDP RX match, `observe_net_probe` after ICMP); smoke greps; `#[test_case] virtio_net_udp_dns_probe`.
- Docs: this ADR; [track-n.md](../04-roadmap/track-n.md); SUMMARY; roadmap; honesty ledger; [limits.md](../overview/limits.md); threat-model bump (**v1.45**); plain overview/architecture; index one-liner.
- Follow-ups: EL0 UDP SVC / TCP / sockets ABI — new ADR only.
