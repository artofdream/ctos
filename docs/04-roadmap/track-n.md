# Track N — network foundation (virtio-net first frame)

Scope ADR: [ADR-063](../03-adr/ADR-063-network-foundation-scope.md). Separate from [Track A](track-a.md) (freestanding apps) and [Track B](track-b.md) (Linux-compat research — **never** implementation).

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Networking must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “has networking” headline.

- Honesty ledger — Verified only with a probe; N0 is **Planned** for runtime claims
- Antifragility — fail-closed smoke + ratchets; do not break blk/FAT markers
- Security — threat model names the I/O surface; no “secure OS”
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

## Goal

Bring up a **minimal** guest network path on QEMU `virt`: virtio-mmio **virtio-net**, discover the device, TX/RX **one raw Ethernet frame**, with serial markers and fail-closed smoke. Reuse virtio-mmio patterns from blk ([ADR-028](../03-adr/ADR-028-virtio-blk-fat16.md)).

## Keep

Honesty ledger, fail-closed `qemu-smoke`, existing virtio-blk + FAT16 + sample catalog markers, QEMU `virt` + AArch64 until an ADR widens it, Track A freestanding path.

## Mile ladder

| ID | Work | Probe that closes it | Status |
| --- | --- | --- | --- |
| N0 | Scope ADR + docs wiring | Docs-build; ledger Planned row; no Verified invent | **Accepted (docs)** — [ADR-063](../03-adr/ADR-063-network-foundation-scope.md) |
| N1 | Link bring-up / first frame | Discover + TX/RX one raw Ethernet frame; QEMU `-netdev` / `-device virtio-net-device`; smoke greps; keep blk/FAT green | **Planned** |
| N2 | Optional ARP + ICMP ping | Named markers + fail-closed smoke | **Planned** (optional) |
| N3 | Transport / sockets | New sponsor scope + new ADR only | **Locked out** |
| N4 | Freestanding sample using net SVCs | After N1+ Verified | **Planned** (after N1+) |

## Explicit non-goals (locked until new ADR + sponsor)

TCP/UDP stack as product; BSD sockets as product; DHCP/DNS as product; Wi‑Fi; virtio-pci-only foundation stories; Linux net stack; “has networking” marketing; EL0 net ABI until after link bring-up Verified.

## Honesty

Do **not** say the guest “has networking” from this page alone. N0 is scope. Runtime claims stay **Planned** until N1 probes. CloudAgent HELD; evidence tags agent-box / EVO-X2 when N1 lands. No self-merge ([ADR-002](../03-adr/ADR-002-pr-identity-split.md)).
