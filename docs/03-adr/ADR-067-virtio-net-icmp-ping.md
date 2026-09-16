# ADR-067 — Track N / N2 — ICMP ping after ARP (kernel-path)

- Status: Accepted (N2 runtime — **Verified** only when this tip’s `qemu-smoke` prints `net: ping-ok`; else Documented/partial)
- Date: 2026-09-16

## Context

[ADR-063](ADR-063-network-foundation-scope.md) opened **Track N**. [ADR-066](ADR-066-virtio-net-first-frame.md) closed **N1**: virtio-net discover + ARP request/reply vs QEMU SLIRP gateway `10.0.2.2` (`net: ok`). This mile is **N2**: after (or with) that ARP exchange, send an **ICMP echo request** from guest `10.0.2.15` to gateway `10.0.2.2` and receive the echo reply — still on the **kernel path** only. Reuse N1 queues/DMA. Do not break blk/FAT/slot/`net: ok` greps.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Keep N1 ARP. Capture gateway MAC from ARP sha. Post a new RX buffer. TX Ethernet+IPv4+ICMP echo request (type 8) to `10.0.2.2`. Wait for ICMP echo reply (type 0) with matching id/seq. Markers + fail-closed smoke. QEMU args **unchanged**: `-netdev user,id=net0 -device virtio-net-device,netdev=net0`. | Deterministic on GHA/agent-box (SLIRP answers ICMP locally). Proves L3 echo without a TCP/UDP stack. |
| **H2 (rejected)** | Add an EL0 net SVC / freestanding ping sample this mile. | N4 territory; prefer kernel-path ping only (sponsor). |
| **H3 (rejected)** | Depend on DHCP/DNS product, outbound Internet, TCP/UDP, or sockets. | Locked non-goals ([ADR-063](ADR-063-network-foundation-scope.md)). |
| **H4 (rejected)** | Invent Verified without `net: ping-ok` on a named tip. | Honesty. |
| **H5 (rejected)** | Drop or weaken N1 ARP markers. | Keep `net: virtio` / `net: mac` / `net: tx` / `net: rx` / `net: ok`. |

## Decision

1. **Order.** Run N1 ARP first (learn gateway MAC from ARP sha). Then ICMP echo TX/RX on the same virtio-net device. Same poll-`used.idx` / identity-PA DMA pattern as N1.

2. **Packet.** Ethernet (dst = gateway MAC, src = guest MAC, ethertype IPv4) + IPv4 (src `10.0.2.15`, dst `10.0.2.2`, proto ICMP, TTL 64) + ICMP echo request (type 8, code 0, fixed id/seq, small payload `ctos-n2`). Ones-complement checksums for IP header and ICMP message. No IP options. No TCP/UDP.

3. **Honest probe.** Require ICMP echo **reply** (type 0) from `10.0.2.2` to `10.0.2.15` with matching id/seq. Host `-netdev` without guest markers is **not** a probe. QEMU user-netdev args stay as in ADR-066 unless a future ADR names a change (none needed here).

4. **Serial markers (N2).**
   - `net: icmp-tx` — TX used ring advanced for the ICMP echo request
   - `net: icmp-rx` — RX delivered a matching ICMP echo reply
   - `net: ping-ok` — N2 closed
   - Keep N1 markers including `net: ok`
   - Miss → `net: probe missed` (fail-closed)
   - Optional cheap CNTPCT: `perf: net-icmp-tx ticks=<n>` / `perf: net-icmp-rx ticks=<n>` (lab only)

5. **Smoke.** `scripts/qemu-smoke.sh` greps N1 markers **and** `net: icmp-tx` / `net: icmp-rx` / `net: ping-ok`. Must keep `blk:*` / `fat:*` / slot / sample / perf greps green.

6. **Non-goals restated (locked).** TCP/UDP product; BSD sockets; DHCP/DNS product; Wi‑Fi; virtio-pci-only foundation; Linux net stack; “has networking” marketing; EL0 net SVC/ABI this PR (N4 later). N3+ needs new sponsor ADR.

7. **Honesty.** Say “the guest ARP’d the SLIRP gateway and completed one ICMP echo request/reply” only when serial/`qemu-smoke` print `net: ping-ok` on a named tip. Do not say the OS “has networking.” CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: extend `src/virtio.rs` (`fill_icmp_echo_request`, ICMP RX match, `observe_net_probe` after ARP); smoke greps; `#[test_case] virtio_net_icmp_echo_ping` (+ keep ARP test).
- Docs: this ADR; [track-n.md](../04-roadmap/track-n.md); SUMMARY; roadmap; honesty ledger; [limits.md](../overview/limits.md); threat-model bump; plain overview/architecture mermaid; index one-liner.
- Follow-ups: N3 sockets / N4 freestanding net sample locked out until new ADR + sponsor.
