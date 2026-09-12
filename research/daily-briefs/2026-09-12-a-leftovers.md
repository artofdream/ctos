# Daily brief — 2026-09-12 (Track A leftovers / ADR-032)

## Where we stopped

Ready PR https://github.com/artofdream/ctos/pull/64 (`cursor/track-a-leftovers-d2ee`) rebased onto `main` `522c543` (#63 mobile theme, plus #62 ADR-031 Linux-compat + #61). One PR: [ADR-032](../../docs/03-adr/ADR-032-track-a-leftovers.md) leftover mile. ADR-031 stays Linux-compat. No new FR/NFR IDs. MRC asked for this rebase because #63 made the PR dirty again.

Cloud `scripts/qemu-smoke.sh` **Verified** on `800f52d` (QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`, `-cpu cortex-a57`):

- Host OS `perf: elf-size bytes=4410056`; app `8624` bytes; `sha256=142dedf1c3f6452555fe3af86c8bfafa260f083fecfdc81159063c433872d865`
- A1–A9 markers still present (`svc` / `libctos` / `loader` / `el0` / `fs` / `blk` / `fat` / `slot` / `ident` / `pan`)
- Embed-off: `kernel does not include_bytes! hello-libctos`; A2–A4 + A9 read FAT `/hello`
- Cross-update: same app on this OS `800f52d` **and** prior OS `ba6541c` (built from that SHA in `/tmp`, not a stored blob); both `slot: ok`
- Isolation honesty: `ident: data-stay lo=0x40201000 hi=0x402042c8` / `ident: heap-stay lo=0x40258000 hi=0x40268000`; `pan: id=0` / `pan: absent`
- `Running 83 tests` all `[ok]`; force-fail exit 1; `qemu-smoke: ok`

## Do next

1. Author does not merge (ADR-002). GitHub author of #64 is **`artofdream`**; merge hat is **`cursor[bot]`**. Owner / this PAT must not merge. MRC grepped `7b68d08`. Later ledger-only SHA is not that probe.
2. Bugbot Mediums on `7b68d08`: Docker lacked `git` / `.git`; hash required `sha256sum`. Source ratchet landed. Docker leftover path and Darwin `shasum` stay **Unknown** until probed.
3. Still Planned: PAN enable; identity `.data`/heap tear (SP + allocator still identity); lower-EL IRQ while standing; EL0 without `TLBI VMALLE1`; umbrella isolation; slot-load **delta**; product app hosting.

## Honesty

- Cloud Verified is this QEMU virt guest on `800f52d`, re-probed on `17778ca` and post-#62 `6abcb1b`. GHA Verified on `17778ca` is push [34701455001](https://github.com/artofdream/ctos/actions/runs/34701455001). GHA Verified on `7b68d08` (after #63) is push [34702218168](https://github.com/artofdream/ctos/actions/runs/34702218168) + PR [34702220491](https://github.com/artofdream/ctos/actions/runs/34702220491). Later ledger-only SHA is not that probe.
- Did not claim “EL0 isolated,” PAN enable, “apps update independently,” or “app hosting is done.”
