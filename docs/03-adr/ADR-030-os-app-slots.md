# ADR-030 — OS image vs app payload slots

- Status: Accepted (first-cut two-artifact + FAT load path). Cross-update is [ADR-032](ADR-032-track-a-leftovers.md) (same published ELF on this OS and `ba6541c`).
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding app that is **not** rebuilt every time the kernel is. [Issue #48](https://github.com/artofdream/ctos/issues/48) is this mile: **disconnect OS image updates from app payloads**.

The sponsor goal is scoped immutability, not an “immutable OS” product sentence ([immutability.md](../framework/immutability.md)). A1–A8 already landed ABI, `libctos`, a guest `PT_LOAD` loader, standing EL0, memfs, virtio-blk + FAT16, and sample recipes. Today the hello ELF is still `include_bytes!` into the kernel. That is one linked image.

This mile must not invent OTA, A-B flash, or OCI containers ([ADR-029](ADR-029-containers-nongoal.md)). No new FR/NFR IDs.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Two host artifacts (kernel ELF + `hello-libctos.elf`) plus a FAT16 **app slot** (`/hello`). Guest `vfs::open` + A3 `PT_LOAD` map. A2–A4 kept the embed **on the first cut**. | Honest first cut. Storage is the existing memfs/raw-image story (A6/A7). Cross-update and embed removal are [ADR-032](ADR-032-track-a-leftovers.md). |
| **H2 (first-cut reject; leftover accept)** | Remove `include_bytes!` and load A2–A4 from FAT only. | Rejected on the first cut (PROBE-only image risk). Leftover mile takes this after smoke already requires `--app`. |
| **H3 (rejected)** | Second QEMU `-kernel`, fw_cfg, or a custom raw slot format. | Extra boot contract. FAT16 + VFS is already probed. |
| **H4 (rejected)** | Claim cross-update Verified because the SVC numbers did not change. | ABI stability is not a two-boot probe. File presence of this ADR is not that probe. |
| **H5 (rejected)** | OTA / A-B flash / OCI layers. | Out of scope. Not Android. Not a container host. |

## Decision

1. **Slots.** The **OS image** is the kernel ELF QEMU `-kernel`s (`target/aarch64-ctos/debug/ctos`). The **app payload** is a separate host ELF (`target/hello-libctos.elf`) built from `user/hello-libctos`. The guest app slot is FAT16 `/hello` (8.3 `HELLO`) on the existing virtio-blk volume. `/probe` stays the A7 known file. Same VFS `open` / `read` / `close`. No new SVC numbers.
2. **Build.** `build.rs` compiles the hello and **publishes** the ELF to `target/hello-libctos.elf` (and still writes host flatten metadata). After [ADR-032](ADR-032-track-a-leftovers.md) the kernel does not `include_bytes!` that file. `scripts/mkfat16.py --app` writes that ELF as `/hello` (cluster chain; cap 64 KiB). Smoke and the QEMU runner fail closed if the app artifact is missing.
3. **Boot/load.** Hello path: OS boots, A1–A8 probes run. After [ADR-032](ADR-032-track-a-leftovers.md), A2–A4 also read FAT `/hello` (no `include_bytes!`). Then A9 reads `/hello` through the thin VFS, parses ELF64 `PT_LOAD` with the A3 loader, and `ERET`s. **No embed fallback** on this path. Missing `/hello` prints `slot: probe missed`.
4. **Fail-closed probe.** Serial `slot: fat` + `slot: mapped` + `slot: ok` after a successful FAT load + `ERET`. `perf: app-load ticks=<n>` is a CNTPCT measurement around that path (NFR-07) — not a bench and not a delta vs the embed. `scripts/qemu-smoke.sh` greps those and rejects `slot: probe missed` / `slot: embed`. Host smoke requires both artifacts and `mkfat16.py --check --require-app`. `#[test_case]` covers FAT `/hello` ELF magic + parse + the FAT load trip.
5. **Honesty.** Say “the guest loaded a separate app ELF from the FAT slot into user TTBR0” only when the serial / tests pass. Do **not** say: apps update independently of the OS, immutable OS, OTA, A-B, containers, “EL0 isolated,” or “secure OS.” Cross-update is a **host** two-boot probe ([ADR-032](ADR-032-track-a-leftovers.md)): same published `hello-libctos.elf` on this OS and on `ba6541c`. File presence of this ADR is not that probe. A kernel rebuild still compiles the payload (`build.rs`); it no longer embeds the bytes.
6. **Performance.** Expected costs stay the documented shape (boot/load, SVC, ASID/TTBR). The new marker proves the path is measurable. A Verified *delta* vs the linked-in trip stays Planned. Do not invent a bench.
7. **NFR-10 text** is revised in place (ID unchanged) to name this slot first cut. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/slot.rs`, `src/loader.rs` (`run_image`), `build.rs` publish path, `scripts/mkfat16.py --app`, QEMU runner / smoke.
- Docs: [immutability.md](../framework/immutability.md), [advantages.md](../overview/advantages.md), [track-a.md](../04-roadmap/track-a.md), threat-model v1.16. Roadmap cites #48 / Track A #31.
- Track A children A1–A9 now have a first cut. [ADR-032](ADR-032-track-a-leftovers.md) takes cross-update + embed-off + isolation honesty. Remaining Planned: PAN enable, remaining identity RAM / `_start`, umbrella isolation, slot-load **delta**, “app hosting is done.”
