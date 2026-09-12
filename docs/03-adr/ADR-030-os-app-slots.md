# ADR-030 — OS image vs app payload slots

- Status: Accepted (first-cut two-artifact + FAT load path when the serial / tests pass; cross-update stays Planned)
- Date: 2026-09-12

## Context

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) wants a freestanding app that is **not** rebuilt every time the kernel is. [Issue #48](https://github.com/artofdream/ctos/issues/48) is this mile: **disconnect OS image updates from app payloads**.

The sponsor goal is scoped immutability, not an “immutable OS” product sentence ([immutability.md](../framework/immutability.md)). A1–A8 already landed ABI, `libctos`, a guest `PT_LOAD` loader, standing EL0, memfs, virtio-blk + FAT16, and sample recipes. Today the hello ELF is still `include_bytes!` into the kernel. That is one linked image.

This mile must not invent OTA, A-B flash, or OCI containers ([ADR-029](ADR-029-containers-nongoal.md)). No new FR/NFR IDs.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Two host artifacts (kernel ELF + `hello-libctos.elf`) plus a FAT16 **app slot** (`/hello`). Guest `vfs::open` + A3 `PT_LOAD` map. A2–A4 keep the embed so their markers stay. | Honest first cut. Storage is the existing memfs/raw-image story (A6/A7). Cross-update (same app on OS n and n+1) stays Planned until that probe exists. |
| **H2 (rejected)** | Remove `include_bytes!` this mile and load A2–A4 from FAT only. | Would risk A1–A8 markers on hosts that attach a PROBE-only image. Keep the embed as the A2–A4 ratchet; A9 is a *second* load path. |
| **H3 (rejected)** | Second QEMU `-kernel`, fw_cfg, or a custom raw slot format. | Extra boot contract. FAT16 + VFS is already probed. |
| **H4 (rejected)** | Claim cross-update Verified because the SVC numbers did not change. | ABI stability is not a two-boot probe. File presence of this ADR is not that probe. |
| **H5 (rejected)** | OTA / A-B flash / OCI layers. | Out of scope. Not Android. Not a container host. |

## Decision

1. **Slots.** The **OS image** is the kernel ELF QEMU `-kernel`s (`target/aarch64-ctos/debug/ctos`). The **app payload** is a separate host ELF (`target/hello-libctos.elf`) built from `user/hello-libctos`. The guest app slot is FAT16 `/hello` (8.3 `HELLO`) on the existing virtio-blk volume. `/probe` stays the A7 known file. Same VFS `open` / `read` / `close`. No new SVC numbers.
2. **Build.** `build.rs` still emits the A2 flattened `.bin` and the A3 embed. It also **publishes** the ELF to `target/hello-libctos.elf`. `scripts/mkfat16.py --app` writes that ELF as `/hello` (cluster chain; cap 64 KiB). Smoke and the QEMU runner fail closed if the app artifact is missing.
3. **Boot/load.** Hello path: OS boots, A1–A8 probes run as today (embed still used for `libctos:*` / `loader:*` / `el0: task-*`). Then A9 reads `/hello` through the thin VFS, parses ELF64 `PT_LOAD` with the A3 loader, and `ERET`s. **No embed fallback** on this path. Missing `/hello` prints `slot: probe missed`.
4. **Fail-closed probe.** Serial `slot: fat` + `slot: mapped` + `slot: ok` after a successful FAT load + `ERET`. `perf: app-load ticks=<n>` is a CNTPCT measurement around that path (NFR-07) — not a bench and not a delta vs the embed. `scripts/qemu-smoke.sh` greps those and rejects `slot: probe missed` / `slot: embed`. Host smoke requires both artifacts and `mkfat16.py --check --require-app`. `#[test_case]` covers FAT `/hello` ELF magic + parse + the FAT load trip.
5. **Honesty.** Say “the guest loaded a separate app ELF from the FAT slot into user TTBR0” only when the serial / tests pass. Do **not** say: apps update independently of the OS, cross-update Verified, immutable OS, OTA, A-B, containers, “EL0 isolated,” or “secure OS.” A2–A4 still embed a copy — the kernel ELF is not app-free. Cross-update (same `hello-libctos.elf` on OS n and a documented prior OS) stays **Planned**.
6. **Performance.** Expected costs stay the documented shape (boot/load, SVC, ASID/TTBR). The new marker proves the path is measurable. A Verified *delta* vs the linked-in trip stays Planned. Do not invent a bench.
7. **NFR-10 text** is revised in place (ID unchanged) to name this slot first cut. Do not mint FR-16+ or NFR-15+.

## Consequences

- Code: `src/slot.rs`, `src/loader.rs` (`run_image`), `build.rs` publish path, `scripts/mkfat16.py --app`, QEMU runner / smoke.
- Docs: [immutability.md](../framework/immutability.md), [advantages.md](../overview/advantages.md), [track-a.md](../04-roadmap/track-a.md), threat-model v1.16. Roadmap cites #48 / Track A #31.
- Track A children A1–A9 now have a first cut. Remaining honesty gaps: cross-update, kernel still embeds A2–A4, PAN enable, identity `.data`/heap tear, umbrella isolation, “app hosting is done.”
