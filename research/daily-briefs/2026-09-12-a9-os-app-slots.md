# Daily brief — 2026-09-12 (Track A / A9 OS vs app slots)

## Where we stopped

PR on `cursor/a9-os-app-slots-cf00` — Track A / A9 / [ADR-030](../../docs/03-adr/ADR-030-os-app-slots.md). Parent [#31](https://github.com/artofdream/ctos/issues/31), child [#48](https://github.com/artofdream/ctos/issues/48). Last Track A child.

**First cut (not cross-update):**

- Host artifacts: OS ELF (`target/aarch64-ctos/debug/ctos`) + published `target/hello-libctos.elf`
- FAT16 `/hello` (8.3 `HELLO`) via `scripts/mkfat16.py --app`
- Guest `src/slot.rs` reads that path through the thin VFS and maps with the A3 loader
- Serial `slot: fat` / `slot: mapped` / `slot: ok` / `perf: app-load`
- A1–A8 markers preserved. A2–A4 still `include_bytes!` (no embed fallback on the A9 path)

Cloud `./scripts/qemu-smoke.sh` **Verified** on `9ca7372` (QEMU 8.2.2, `rustc` 1.100.0-nightly `0fc141305`): host OS `4392040` bytes + app `8624` bytes; `slot: fat bytes=8624` / `slot: mapped` / `slot: ok` / `perf: app-load ticks=1134019`; A1–A8 markers present; `Running 82 tests` all `[ok]`; force-fail exit 1. Cross-update stays **Planned**. Do not say “apps update independently” or “app hosting is done.”

## Do next

1. Human or MRC review. Author does not merge (ADR-002).
2. After smoke green, mark the draft PR ready. Distinct merger.
3. Remaining Track A honesty gaps: A9 cross-update; kernel still embeds A2–A4; PAN enable; identity `.data`/heap tear; umbrella isolation; product app hosting.

## Honesty

- File presence of ADR-030 is not QEMU boot.
- `perf: app-load` is a measurement, not a delta vs the embed.
