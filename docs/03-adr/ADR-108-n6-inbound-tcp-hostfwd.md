# ADR-108 — Track N / N6 — inbound TCP (one passive open over QEMU hostfwd)

- Status: Accepted (runtime **Verified** only when this tip's `qemu-smoke` prints `net: tcp-in-ok` and the host client prints `hostfwd-client: neg=reset pos=ok rx=19 eof=yes`; otherwise Documented)
- Date: 2026-10-23

## Context

[ADR-074](ADR-074-n5-thin-tcp.md) (N5, PR #120) gave the kernel a thin **active-open** TCP: SYN to a QEMU `guestfwd` echo at 10.0.2.4:7, one payload, one echo. [ADR-076](ADR-076-el0-tcp-svc.md) exposed it to EL0, and [ADR-078](ADR-078-tcp-echo-cntpct.md) timed it. ADR-074 listed listen/accept as a locked non-goal. The sponsor now asks for **inbound** TCP: the host connects, data is exchanged, the connection closes, with fail-closed checks and a negative, over whatever networking the smoke already uses (QEMU user-net).

With QEMU user-net, `hostfwd=tcp:127.0.0.1:<host port>-:<guest port>` makes SLIRP accept on host loopback and open a TCP connection to the guest (source 10.0.2.2, destination 10.0.2.15:<guest port>). SLIRP is a full TCP peer: it retransmits, ACKs and closes like a normal stack.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Kernel-path passive open on one port (8080) via `hostfwd`. The host client in `qemu-serial-inject.py` waits for the guest cue `net: tcp-in-listen`, then runs the negative and the positive. | Deterministic in CI (host loopback only, no external net). It uses the same virtio-net path and SLIRP as N1–N5. |
| **H2 (rejected)** | A sockets ABI (`listen`/`accept`/`recv`/`send` SVCs) with a socket table. | Product-sized. An EL0 surface can follow in its own ADR. |
| **H3 (rejected)** | TAP/bridge networking for a "real" inbound path. | Needs host privileges in CI and changes the network the smoke uses. |
| **H4 (rejected)** | Connect blindly on a timer without a guest cue. | Racy: SLIRP would accept on the host before the guest polls, and the result would depend on retransmit timing. |

## Decision

1. **QEMU.** The hello-boot runner (`qemu-serial-inject.py`) adds `hostfwd=tcp:127.0.0.1:<p1>-:8080,hostfwd=tcp:127.0.0.1:<p2>-:8081` to the existing user netdev. `p1` and `p2` are free loopback ports chosen at run time; nothing binds outside loopback. Guest port 8081 has no listener: it is the negative.
2. **Guest (`src/tcpin.rs`, after the N1–N5 probe).** Two small raw-frame helpers are added to `virtio.rs`: `net_raw_tx` and `net_raw_rx`. `net_raw_rx` reuses a still-posted RX buffer rather than posting twice. The guest then does the following:
   - **Cue.** Print `net: tcp-in-listen port=8080 closed=8081`, then poll for up to 15 s.
   - **ARP.** Answer ARP who-has 10.0.2.15.
   - **Parse.** Accept only IPv4/TCP to 10.0.2.15 with a valid TCP checksum. Bad-checksum segments are dropped and counted; fragments and other addresses are dropped.
   - **Closed port.** A SYN to 8081 gets an RFC 793 reset (`RST|ACK`, `ack = seq + 1`) and `net: tcp-in-rst port=8081`. Late segments of earlier outbound probes, which arrive at other ports, also get an RST but do not count as the negative.
   - **Port 8080, one peer.**
     1. SYN → SYN-ACK (`net: tcp-in-syn`; a retransmitted SYN gets the SYN-ACK again).
     2. The ACK of our ISS+1 → `net: tcp-in-est`.
     3. In-order data up to a newline → `net: tcp-in-rx bytes=N`. Out-of-order data is re-ACKed.
     4. Reply `ctos-in:<line>` (`net: tcp-in-tx bytes=M`), then the guest closes with FIN.
     5. The host's ACK of our FIN plus the host's FIN, which we ACK → `net: tcp-in-fin closed`.

     A second 4-tuple to 8080 gets an RST. A peer RST is a failure.
   - **Result.** `net: tcp-in-ok` requires both the completed exchange and the closed-port RST. Otherwise the guest prints `net: tcp-in-fail reason=…` and the kernel prints `net: tcp-in probe missed`.
3. **Host client.** It starts on the cue and runs two steps:
   1. **Negative.** Connect to `p2`. It must read EOF or get a reset with zero bytes; a timeout or any data is a failure.
   2. **Positive.** Connect to `p1`, send `ctos-hello\n`, and read to EOF. The reply must be exactly `ctos-in:ctos-hello\n` (19 bytes). Then close.

   It prints `hostfwd-client: neg=… pos=… rx=… eof=…` after QEMU's output.
4. **Smoke (fail closed).** `qemu-smoke.sh` rejects any `tcp-in-fail` or `probe missed`. It requires all eight guest markers (including `rx bytes=11`, `tx bytes=19` and `rst port=8081`) and the exact host line `hostfwd-client: neg=reset pos=ok rx=19 eof=yes`.
5. **`cargo test` (in QEMU).** The tests cover:
   - parsing a SYN;
   - a corrupted segment being rejected as `BadChecksum`;
   - RFC 793 RST fields for a SYN and for an ACK, and never answering an RST;
   - reply checksums being valid with addresses and ports swapped;
   - fragments and foreign destinations being dropped.

   The runtime exchange itself is only in the boot smoke, which is where the host client exists.

## Not claimed

- No sockets API, no EL0 SVC, no backlog or socket table, no concurrent connections, no retransmission timer, no window management or options (MSS not sent), no urgent data, no IPv6, no TLS or HTTP.
- No real-network claim: SLIRP on host loopback only, QEMU `virt` TCG only. Not a security boundary.
- N5 outbound TCP (ADR-074/076/078) is unchanged.

## Consequences

- Every hello boot through `qemu-serial-inject.py` now runs the inbound exchange; the host client is always present there. `cargo test` boots (`qemu-aarch64.sh`) have no hostfwd and do not run this boot-path probe.
- A later ADR can put an EL0 `accept`/`recv`/`send` surface on top of this state machine.
