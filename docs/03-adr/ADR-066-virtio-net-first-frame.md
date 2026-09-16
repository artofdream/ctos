# ADR-066 — Track N / N1 — virtio-net first Ethernet frame

- Status: Accepted (N1 runtime — **Verified** only when this tip’s `qemu-smoke` prints `net: ok`; else Documented/partial)
- Date: 2026-09-16

## Context

[ADR-063](ADR-063-network-foundation-scope.md) opened **Track N** (docs N0). This mile is **N1**: discover virtio-net on virtio-mmio, program RX+TX queues enough to send and receive **one raw Ethernet frame**, with serial markers, fail-closed smoke, and host QEMU `-netdev` / `-device virtio-net-device`. Reuse virtio-mmio patterns from blk ([ADR-028](ADR-028-virtio-blk-fat16.md), `src/virtio.rs`). Do not break FAT/blk/slot/sample/perf greps.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Discover Device ID 1 on the same mmio scan as blk. Setup queue 0 (RX) + queue 1 (TX). Post one RX buffer. TX a broadcast **ARP request** for QEMU SLIRP gateway `10.0.2.2` from guest `10.0.2.15`. Wait for TX `used.idx`, then RX ARP **reply** from SLIRP. Markers + smoke. Host: `-netdev user,id=net0 -device virtio-net-device,netdev=net0`. | Deterministic on GHA ubuntu (no external network). Proves both directions. |
| **H2 (rejected as primary)** | TX-only: prove `used.idx` after TX without RX. | Allowed as a fallback minimum by ADR-063, but SLIRP ARP reply is available and stronger. Prefer both. |
| **H3 (rejected)** | Depend on DHCP/DNS product, outbound Internet, or a second peer VM. | Flaky / product creep. Fail-closed CI must not need external net. |
| **H4 (rejected)** | Inject a crafted RX via QMP / filter without guest TX. | Extra host machinery; ARP round-trip is enough and exercises TX+RX. |
| **H5 (rejected)** | virtio-pci-only, Wi‑Fi, TCP/UDP stack, BSD sockets, EL0 net SVC this mile. | Locked non-goals ([ADR-063](ADR-063-network-foundation-scope.md)). |
| **H6 (rejected)** | Invent Verified without `net: ok` on a named tip. | Honesty. |

## Decision

1. **Transport.** Same virtio-mmio scan (`0x0a000000 + n×0x200`, 32 slots). Device ID **1** = net. Accept legacy v1 (`QueuePFN`) and modern v2 (desc/avail/used + `QueueReady`), matching blk. Identity-PA DMA + whole-object `dc civac` cache maintenance. Poll `used.idx`. No virtio IRQ this mile.

2. **Queues.** Queue 0 = receiveq, queue 1 = transmitq. Separate 4 KiB-aligned `.bss` DMA objects (legacy page layout). 10-byte `virtio_net_hdr` (no `VIRTIO_NET_F_MRG_RXBUF`). Negotiate `VIRTIO_NET_F_MAC` when offered; read MAC from config bytes 0–5.

3. **Honest probe (CI-deterministic).**
   - Post one RX WRITE buffer (hdr+frame).
   - TX Ethernet+ARP request: who-has **10.0.2.2** tell **10.0.2.15**, src = guest MAC, dst = broadcast.
   - QEMU **user** netdev SLIRP answers ARP for its gateway locally — no Internet, no DHCP client, no second VM.
   - Require TX completion **and** an ARP reply whose spa is `10.0.2.2`.

4. **Host QEMU args (exact).** Wired into `scripts/qemu-aarch64.sh` and `scripts/qemu-serial-inject.py` (hello smoke path):

   ```
   -netdev user,id=net0
   -device virtio-net-device,netdev=net0
   ```

   Kept alongside existing `-drive … -device virtio-blk-device,drive=hd0`. Host flags without guest markers are **not** a probe.

5. **Serial markers.**
   - `net: virtio` — discovered + queues programmed
   - `net: mac xx:xx:xx:xx:xx:xx` — config MAC
   - `net: tx` — TX used ring advanced for the ARP request
   - `net: rx` — RX used ring delivered an ARP reply from `10.0.2.2`
   - `net: ok` — both directions proven
   - Miss → `net: probe missed` (fail-closed)
   - Optional cheap CNTPCT: `perf: net-tx ticks=<n>` / `perf: net-rx ticks=<n>` when the counter advances (lab only, not a bench)

6. **Smoke.** `scripts/qemu-smoke.sh` greps the markers above and rejects `net: probe missed`. Must keep existing `blk:*` / `fat:*` / slot / sample / perf greps green.

7. **Non-goals restated (locked).** TCP/UDP product; BSD sockets; DHCP/DNS product; Wi‑Fi; virtio-pci-only foundation; Linux net stack; “has networking” marketing; EL0 net SVC/ABI. N2 ICMP is [ADR-067](ADR-067-virtio-net-icmp-ping.md); N3+ needs new sponsor ADR.

8. **Honesty.** Say “the guest programmed virtio-net and exchanged one ARP request/reply with QEMU SLIRP” only when serial/`qemu-smoke` print `net: ok` on a named tip. Do not say the OS “has networking.” CloudAgent HELD; tag agent-box / EVO-X2. No self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

## Consequences

- Code: extend `src/virtio.rs` (net discover/setup/TX/RX + `observe_net_probe`); `src/main.rs` calls `init_net` + probe; QEMU runners + smoke greps; `#[test_case] virtio_net_discover_and_arp`.
- Docs: this ADR; [track-n.md](../04-roadmap/track-n.md); SUMMARY; roadmap; honesty ledger; [limits.md](../overview/limits.md); threat-model bump; index one-liner.
- Follow-ups: N2 ICMP ping — [ADR-067](ADR-067-virtio-net-icmp-ping.md); N3/N4 locked out until new ADR + sponsor.
