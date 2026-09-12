# Daily brief — 2026-09-12 (Track A leftovers / ADR-031)

## Where we stopped

Draft-then-ready PR https://github.com/artofdream/ctos/pull/64 (`cursor/track-a-leftovers-d2ee`). Parent is `main` `ba6541c` (A9 / ADR-030). One PR: [ADR-031](../../docs/03-adr/ADR-031-track-a-leftovers.md) leftover mile. No new FR/NFR IDs.

Cloud `scripts/qemu-smoke.sh` **Verified** on `800f52d` (QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`, `-cpu cortex-a57`):

- Host OS `perf: elf-size bytes=4410056`; app `8624` bytes; `sha256=142dedf1c3f6452555fe3af86c8bfafa260f083fecfdc81159063c433872d865`
- A1–A9 markers still present (`svc` / `libctos` / `loader` / `el0` / `fs` / `blk` / `fat` / `slot` / `ident` / `pan`)
- Embed-off: `kernel does not include_bytes! hello-libctos`; A2–A4 + A9 read FAT `/hello`
- Cross-update: same app on this OS `800f52d` **and** prior OS `ba6541c` (built from that SHA in `/tmp`, not a stored blob); both `slot: ok`
- Isolation honesty: `ident: data-stay lo=0x40201000 hi=0x402042c8` / `ident: heap-stay lo=0x40258000 hi=0x40268000`; `pan: id=0` / `pan: absent`
- `Running 83 tests` all `[ok]`; force-fail exit 1; `qemu-smoke: ok`

## Do next

1. Human or MRC review. Author does not merge (ADR-002). GitHub author of #64 is `cursor[bot]`; merge hat is **`artofdream`**.
2. Still Planned: PAN enable; identity `.data`/heap tear (SP + allocator still identity); lower-EL IRQ while standing; EL0 without `TLBI VMALLE1`; umbrella isolation; slot-load **delta**; product app hosting.

## Honesty

- Cloud Verified is this QEMU virt guest on `800f52d`. Later ledger-only SHAs are not that probe.
- Did not claim “EL0 isolated,” PAN enable, “apps update independently,” or “app hosting is done.”
