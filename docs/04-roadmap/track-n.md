# Track N — network foundation (virtio-net ARP + ICMP + UDP + EL0 net/UDP samples)

Scope ADR: [ADR-063](../03-adr/ADR-063-network-foundation-scope.md). Separate from [Track A](track-a.md) (freestanding apps) and [Track B](track-b.md) (Linux-compat research — **never** implementation).

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Networking must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “has networking” headline.

- Honesty ledger — Verified only with a probe; N0 is **Planned** for runtime claims
- Antifragility — fail-closed smoke + ratchets; do not break blk/FAT markers
- Security — threat model names the I/O surface; no “secure OS”
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

## Goal

Bring up a **minimal** guest network path on QEMU `virt`: virtio-mmio **virtio-net**, discover the device, TX/RX **one raw Ethernet frame** (N1), an **ICMP echo** to the SLIRP gateway (N2), then a **UDP datagram** to SLIRP DNS (N3), with serial markers and fail-closed smoke. Reuse virtio-mmio patterns from blk ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). N1: [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md). N2: [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md). N3: [ADR-070](../03-adr/ADR-070-n3-udp-transport.md). N3.x EL0 UDP SVC: [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md).

## Keep

Honesty ledger, fail-closed `qemu-smoke`, existing virtio-blk + FAT16 + sample catalog markers, QEMU `virt` + AArch64 until an ADR widens it, Track A freestanding path.

## Mile ladder

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| N0 | Scope ADR + docs wiring | Docs-build; ledger Planned row; no Verified invent | **Accepted (docs)** — [ADR-063](../03-adr/ADR-063-network-foundation-scope.md) |
| N1 | Link bring-up / first frame | Discover + TX/RX one raw Ethernet frame; QEMU `-netdev` / `-device virtio-net-device`; smoke greps; keep blk/FAT green | **Verified** when this tip’s `qemu-smoke` prints `net: ok` ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md)) |
| N2 | ARP + ICMP ping (kernel-path) | After ARP: ICMP echo request/reply vs `10.0.2.2`; `net: icmp-tx` / `net: icmp-rx` / `net: ping-ok`; keep N1 + blk/FAT green | **Verified** when this tip’s `qemu-smoke` prints `net: ping-ok` ([ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md)) |
| N3 | Transport — minimal UDP (IPv4) | UDP TX/RX vs SLIRP DNS `10.0.2.3:53`; `net: udp-tx` / `net: udp-rx` / `net: udp-ok`; keep N1/N2 + blk/FAT green | **Verified** when this tip’s `qemu-smoke` prints `net: udp-ok` ([ADR-070](../03-adr/ADR-070-n3-udp-transport.md)) |
| N4 | Freestanding sample using net SVCs | After N1+ Verified; EL0 `net_mac`/`net_ping` + FAT `/netdemo`; smoke greps | **Verified** when this tip’s `qemu-smoke` prints `libctos: net-ok` / `netdemo: ok` ([ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md)) |
| N3.x | EL0 UDP DNS SVC + freestanding sample | After N3 Verified; EL0 `net_udp_dns` + FAT `/udpdemo`; smoke greps | **Verified** when this tip’s `qemu-smoke` prints `libctos: udp-ok` / `udpdemo: ok` ([ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md)) |

## Explicit non-goals (locked until new ADR + sponsor)

BSD sockets as product; TCP stack as product; DHCP/DNS as product features (N3/N3.x may *use* SLIRP DNS as probe bait only); Wi‑Fi; virtio-pci-only foundation stories; Linux net stack; “has networking / sockets OS” marketing. N3.x adds EL0 `net_udp_dns` only ([ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md)); general `udp_send`/`udp_recv` / TCP / sockets ABI need another ADR. N4 EL0 net ABI remains the tiny `net_mac`/`net_ping` surface ([ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md)).

## Honesty

Do **not** say the guest “has networking” or “has sockets.” N0 is scope ([ADR-063](../03-adr/ADR-063-network-foundation-scope.md)). N1 first-frame is [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) (`net: ok`). N2 ICMP ping is [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) — Verified only with `net: ping-ok` on a named tip. N3 UDP is [ADR-070](../03-adr/ADR-070-n3-udp-transport.md) — Verified only with `net: udp-ok` on a named tip. N3.x EL0 UDP SVC sample is [ADR-071](../03-adr/ADR-071-n3x-el0-udp-svc.md) — Verified only with `libctos: udp-ok` / `udpdemo: ok` on a named tip. N4 EL0 net sample is [ADR-068](../03-adr/ADR-068-el0-net-svc-sample.md) — Verified only with `libctos: net-ok` / `netdemo: ok` on a named tip. Still not BSD sockets / TCP product / DHCP/DNS product. CloudAgent HELD; evidence tags agent-box / EVO-X2. No self-merge ([ADR-002](../03-adr/ADR-002-pr-identity-split.md)).
