# ADR-063 — Track N — network foundation scope (virtio-net first frame)

- Status: Accepted (docs scope; **Planned** until N1 probes — do not invent Verified)
- Date: 2026-09-16

## Context

Sponsor treats **networking as a new track** (Track N), not a side edit on filesystem / freestanding-sample PRs. Today the guest has virtio-mmio + virtio-blk ([ADR-028](ADR-028-virtio-blk-fat16.md), `src/virtio.rs`) and no NIC / no virtio-net ([limits.md](../overview/limits.md)). Track A freestanding hosting is Verified under [ADR-048](ADR-048-app-hosting-claim-criteria.md) / [ADR-052](ADR-052-sponsor-accept-app-hosting.md). Track B Linux-compat research ended **never** ([ADR-036](ADR-036-linux-compat-decision.md)).

This mile (N0) is **document-first**: name the track, lock foundation scope and non-goals, publish a mile ladder, and wire SUMMARY / roadmap / honesty ledger / limits. No virtio-net driver code in this PR unless trivial scaffolding is needed for honesty — prefer docs-only. File presence of this ADR is **not** a network probe.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Open **Track N** (network) as a separate ladder from Track A/B. N0 = this ADR (scope). Foundation target: QEMU `virt` + virtio-net (mmio); discover device; TX/RX one raw Ethernet frame; serial markers + smoke + QEMU `-netdev` / `-device virtio-net-device`; measure-first; fail-closed. | Matches sponsor “new track” + honesty. Reuses virtio-mmio patterns from blk. |
| **H2 (rejected)** | Fold networking into Track A FS / sample PRs as a side edit. | Sponsor forbids; muddies A7/FAT/smoke ownership. |
| **H3 (rejected)** | Ship a TCP/UDP stack, BSD sockets, DHCP/DNS product, or “has networking” marketing this mile. | Honesty fail. Locked non-goals until a new ADR + sponsor. |
| **H4 (rejected)** | Claim Verified for link bring-up or first frame from this docs PR. | No guest probe yet. Status stays **Planned**. |
| **H5 (rejected)** | virtio-pci-only or Wi‑Fi stories as the foundation path. | Keep learning scope on QEMU `virt` + virtio-mmio (same as blk). |

## Decision

1. **Track name.** **Track N** (network). Separate from Track A (freestanding app hosting) and Track B (Linux-compat research). Subordinate to [principles.md](../framework/principles.md). Hub: [track-n.md](../04-roadmap/track-n.md).

2. **In scope for foundation (N1 and immediate follow-ons under this ADR’s intent).**
   - QEMU `virt` + **virtio-net** over **virtio-mmio** (scan transports like blk; Device ID for net).
   - Discover the device; program queues enough to **TX and RX one raw Ethernet frame**.
   - Serial markers + fail-closed `scripts/qemu-smoke.sh` greps.
   - Host QEMU flags: `-netdev` + `-device virtio-net-device` (exact args named in the N1 PR).
   - **Measure-first** (NFR-07): CNTPCT or similar only when a path exists; no invented benches.
   - **Fail-closed:** missing markers / `probe missed` fail smoke. Do not break existing FAT / blk / slot / sample greps.

3. **Explicit non-goals (locked until a new ADR + sponsor).**
   - TCP/UDP stack as product.
   - BSD sockets API as product.
   - DHCP / DNS as product features.
   - Wi‑Fi.
   - virtio-pci-only stories as the foundation path.
   - Linux network stack / “runs Linux networking.”
   - Marketing “has networking” / “network-ready OS.”
   - EL0 net SVC / ABI until **after** link bring-up is **Verified** (N1+) — unlocked by [ADR-068](ADR-068-el0-net-svc-sample.md).

4. **Mile ladder.**

   | ID | Work | Probe that closes it | Status |
   | --- | --- | --- | --- |
   | **N0** | This ADR — scope, non-goals, wiring | Docs-build; ledger **Planned** row; no Verified invent | **Accepted (docs)** — this PR |
   | **N1** | Link bring-up / first frame | Serial markers for discover + TX and/or RX one raw frame; smoke greps; QEMU `-netdev`/`-device`; keep blk/FAT smoke green | **Verified** via [ADR-066](ADR-066-virtio-net-first-frame.md) when tip prints `net: ok` |
   | **N2** | ARP + ICMP ping | Named markers + fail-closed smoke; still no sockets product | **Verified** via [ADR-067](ADR-067-virtio-net-icmp-ping.md) when tip prints `net: ping-ok` |
   | **N3** | Transport / sockets | Only with **new sponsor scope** + new ADR | **Locked out** until that ADR |
   | **N4** | Freestanding sample using net SVCs | After N1+ Verified; catalog honesty like ADR-059/061/062 | **Verified** via [ADR-068](ADR-068-el0-net-svc-sample.md) when tip prints `libctos: net-ok` / `netdemo: ok` |

5. **Reuse.** Reuse virtio-mmio patterns from blk ([ADR-028](ADR-028-virtio-blk-fat16.md), `src/virtio.rs`): transport scan, split virtqueue, poll used ring, identity-PA DMA + cache maintenance. Do **not** break FAT/blk smoke or A9/slot/sample markers. Prefer extending virtio carefully over a second transport story.

6. **Honesty.**
   - Do **not** invent **Verified** for networking from this PR.
   - Ledger status for Track N foundation / first frame stays **Planned** until N1 probes pass.
   - Do not claim: TCP/UDP, sockets, DHCP/DNS product, Wi‑Fi, Linux net stack, “has networking,” EL0 net ABI, “secure OS,” or reopen Track B Linux-compat.
   - No silent `-cpu`. Never yank `_start`. Author does not self-merge ([ADR-002](ADR-002-pr-identity-split.md)).
   - CloudAgent remains HELD; N1 evidence tags agent-box or EVO-X2 as available.

7. **Docs wiring (this PR).** SUMMARY + ADR list; roadmap Track N section; [track-n.md](../04-roadmap/track-n.md); honesty-ledger **Planned** row; [limits.md](../overview/limits.md) (“Track N Planned — see ADR”); index one-liner; threat-model light bump ([security.md](../framework/security.md) v1.38 — I/O surface names Track N Planned, still no stack).

8. **No new FR/NFR IDs.** Frozen set stays [fr-nfr.md](../02-requirements/fr-nfr.md). Cite existing NFR-06 / NFR-07 / NFR-10 / FR-14 as needed.

## Consequences

- Docs: this ADR; [track-n.md](../04-roadmap/track-n.md); SUMMARY; roadmap; honesty ledger; limits; index; threat-model v1.38.
- Code: **none** in N0 (docs-only). N1 may add virtio-net guest code + QEMU netdev in a later PR.
- Follow-ups: N1 link bring-up / first frame PR; optional N2 ARP+ICMP; N3+ only with new sponsor ADR.
