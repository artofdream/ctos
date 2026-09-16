# Track N — network foundation (virtio-net ARP + ICMP ping)

Scope ADR: [ADR-063](../03-adr/ADR-063-network-foundation-scope.md). Separate from [Track A](track-a.md) (freestanding apps) and [Track B](track-b.md) (Linux-compat research — **never** implementation).

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Networking must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “has networking” headline.

- Honesty ledger — Verified only with a probe; N0 is **Planned** for runtime claims
- Antifragility — fail-closed smoke + ratchets; do not break blk/FAT markers
- Security — threat model names the I/O surface; no “secure OS”
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

## Goal

Bring up a **minimal** guest network path on QEMU `virt`: virtio-mmio **virtio-net**, discover the device, TX/RX **one raw Ethernet frame** (N1), then an **ICMP echo** to the SLIRP gateway (N2), with serial markers and fail-closed smoke. Reuse virtio-mmio patterns from blk ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)). N1: [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md). N2: [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md).

## Keep

Honesty ledger, fail-closed `qemu-smoke`, existing virtio-blk + FAT16 + sample catalog markers, QEMU `virt` + AArch64 until an ADR widens it, Track A freestanding path.

## Mile ladder

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| N0 | Scope ADR + docs wiring | Docs-build; ledger Planned row; no Verified invent | **Accepted (docs)** — [ADR-063](../03-adr/ADR-063-network-foundation-scope.md) |
| N1 | Link bring-up / first frame | Discover + TX/RX one raw Ethernet frame; QEMU `-netdev` / `-device virtio-net-device`; smoke greps; keep blk/FAT green | **Verified** when this tip’s `qemu-smoke` prints `net: ok` ([ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md)) |
| N2 | ARP + ICMP ping (kernel-path) | After ARP: ICMP echo request/reply vs `10.0.2.2`; `net: icmp-tx` / `net: icmp-rx` / `net: ping-ok`; keep N1 + blk/FAT green | **Verified** when this tip’s `qemu-smoke` prints `net: ping-ok` ([ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md)) |
| N3 | Transport / sockets | New sponsor scope + new ADR only | **Locked out** |
| N4 | Freestanding sample using net SVCs | After N1+ Verified | **Planned** (after N1+) |

## Explicit non-goals (locked until new ADR + sponsor)

TCP/UDP stack as product; BSD sockets as product; DHCP/DNS as product; Wi‑Fi; virtio-pci-only foundation stories; Linux net stack; “has networking” marketing; EL0 net ABI until after link bring-up Verified.

## Honesty

Do **not** say the guest “has networking.” N0 is scope ([ADR-063](../03-adr/ADR-063-network-foundation-scope.md)). N1 first-frame is [ADR-066](../03-adr/ADR-066-virtio-net-first-frame.md) (`net: ok`). N2 ICMP ping is [ADR-067](../03-adr/ADR-067-virtio-net-icmp-ping.md) — Verified only with `net: ping-ok` on a named tip. Still not TCP/UDP/sockets/DHCP/DNS. CloudAgent HELD; evidence tags agent-box / EVO-X2. No self-merge ([ADR-002](../03-adr/ADR-002-pr-identity-split.md)).
