# ADR-074 — Track N / N5 — thin TCP unlock (active open + one payload)

- Status: Accepted (N5 runtime — **Verified** only when this tip’s `qemu-smoke` prints `net: tcp-ok`; else Documented/partial)
- Date: 2026-09-16

## Context

[ADR-063](ADR-063-network-foundation-scope.md) opened **Track N**. N1–N4 + N3.x are Verified ([ADR-066](ADR-066-virtio-net-first-frame.md) / [ADR-067](ADR-067-virtio-net-icmp-ping.md) / [ADR-070](ADR-070-n3-udp-transport.md) / [ADR-071](ADR-071-n3x-el0-udp-svc.md) / [ADR-068](ADR-068-el0-net-svc-sample.md)). FAT mkdir depth is [ADR-073](ADR-073-fat16-mkdir.md). Sponsor train **mile (3)** unlocks a **thin / honest TCP** probe — not a BSD sockets product.

Prior miles left TCP as a locked non-goal. This ADR opens a **single-connection kernel-path** mile only.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | After N1 ARP + N2 ICMP + N3 UDP on the same virtio-net path: active-open TCP to a **QEMU `guestfwd`** echo at `10.0.2.4:7` (`cmd:` → `scripts/tcp-echo-stdio.sh`), send one payload `ctos-tcp\n`, require matching echo. Markers `net: tcp-syn` / `net: tcp-est` / `net: tcp-tx` / `net: tcp-rx` / `net: tcp-ok`. | CI-deterministic (no external net). More honest than hoping SLIRP HTTP exists. Documents exact `-netdev` change. |
| **H2 (rejected)** | Rely on SLIRP built-in HTTP / external TCP peer. | Not fail-closed on GHA/agent-box; external net forbidden for smoke. |
| **H3 (rejected)** | Ship a BSD sockets ABI, `listen`/`accept` product, TLS, HTTP stack, or “has TCP sockets OS” marketing. | Honesty fail; locked product non-goals. |
| **H4 (rejected)** | Docs-only unlock with no guest probe. | Sponsor asked for a real thin TCP mile. |
| **H5 (rejected)** | Require EL0 TCP SVC this mile. | Optional follow-up ADR; kernel-path proof matches N2/N3 style. |

## Decision

1. **Unlock N5** under this ADR. Ladder status moves from Locked → **Verified** when this tip’s `qemu-smoke` prints `net: tcp-ok` (with N1–N3 markers preserved).

2. **In scope this mile.**
   - Minimal **TCP active open** (SYN → SYN-ACK → ACK) + **one payload** send/recv on existing virtio-net (reuse ARP/IP helpers).
   - One connection; no socket table; no listen/accept.
   - Honest CI probe via QEMU user-netdev **`guestfwd`**:
     - `-netdev user,id=net0,guestfwd=tcp:10.0.2.4:7-cmd:<repo>/scripts/tcp-echo-stdio.sh`
     - `-device virtio-net-device,netdev=net0` (unchanged device line)
   - Host peer script echoes exactly nine bytes `ctos-tcp\n` then exits (TCP FIN). No commas in the `-netdev` path.
   - Serial markers + fail-closed smoke greps.
   - EL0 TCP SVC: **not** this PR (follow-up ADR only).

3. **Still not product (locked).**
   - Full BSD sockets API; `listen` / `accept` product; multi-connection socket table.
   - TLS; HTTP stack; DHCP / DNS as product features.
   - Wi‑Fi; Linux net stack; marketing “has TCP / sockets OS.”

4. **Packet / state (tiny).**
   - Ethernet dst = gateway MAC from N1 ARP (SLIRP L2 for `10.0.2.4`).
   - IPv4: src `10.0.2.15`, dst `10.0.2.4`, proto TCP, TTL 64.
   - TCP: src ephemeral `0x6334`, dst `7`, fixed ISN `0x10000000`, no options, window `8192`, real TCP checksum (pseudo-header).
   - Payload `ctos-tcp\n` (9 bytes) with PSH+ACK after established.
   - Accept any matching peer segment that carries the echoed payload.

5. **Serial markers (N5).**
   - `net: tcp-syn` — SYN TX used-ring advanced
   - `net: tcp-est` — SYN-ACK seen + ACK TX’d
   - `net: tcp-tx` — payload PSH+ACK TX’d
   - `net: tcp-rx` — matching echo payload RX’d
   - `net: tcp-ok` — N5 closed
   - Keep N1 (`net: ok`) / N2 (`net: ping-ok`) / N3 (`net: udp-ok`)
   - Miss → `net: probe missed` (fail-closed)

6. **Smoke.** `scripts/qemu-smoke.sh` greps N1–N3 **and** the N5 markers. Must keep `blk:*` / `fat:*` / slot / sample / perf greps green. Runners that attach netdev (`qemu-aarch64.sh`, `qemu-serial-inject.py`) **must** include the `guestfwd` clause. Default `CTOS_QEMU_TIMEOUT` raised **12 → 20** so the longer N1–N5 + sample path still finishes under the inject capture window.

7. **Follow-ups.** EL0 TCP SVC / richer socket ABI / listen-accept only with **another ADR** + sponsor.

8. **Honesty.** Say “the guest completed one TCP active open and echoed one payload via QEMU guestfwd” only when serial/`qemu-smoke` print `net: tcp-ok` on a named tip. Do not say the OS “has networking,” “has sockets,” or “has TCP.” CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: extend `src/virtio.rs` (thin TCP TX/RX + `observe_net_probe` after UDP); `scripts/tcp-echo-stdio.sh` + `.py`; QEMU netdev `guestfwd`; smoke greps; `#[test_case] virtio_net_tcp_echo_probe`.
- Docs: this ADR; [track-n.md](../04-roadmap/track-n.md); SUMMARY; roadmap; honesty ledger; [limits.md](../overview/limits.md); threat-model bump (**v1.49**); index / overview one-liners.
- Follow-ups: EL0 TCP SVC — new ADR only.
