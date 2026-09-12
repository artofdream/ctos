# Track A — freestanding app hosting

Tracker epic: [issue #31](https://github.com/artofdream/ctos/issues/31). Child issues A1–A9.

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Do not skip a probe, drop a ratchet, or invent a “secure/fast app runtime” to finish a row faster.

- Honesty ledger — Verified only with a probe
- Antifragility — fail-closed smoke + ratchets
- Security — threat model; probed mitigations only
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

A9 (OS/app slot disconnect) sits **after** A1–A4. It does not override the list above.

## Goal

A **freestanding** (non-POSIX) application can be loaded and run with honest probes. Not Linux containers. Not glibc.

## Ordered work

One loop unit each. Each row needs a fail-closed ledger probe.

| ID | Work | Status (2026-09-11) |
| --- | --- | --- |
| A1 | Stable SVC / syscall ABI + docs | **ABI mile Verified** on `8846bc5` ([#32](https://github.com/artofdream/ctos/issues/32), [ADR-021](../03-adr/ADR-021-svc-syscall-abi.md), [syscall.md](../framework/syscall.md)). Not app hosting. |
| A2 | Freestanding CRT / `libctos` | **CRT mile Verified** on `c9b292b` ([#33](https://github.com/artofdream/ctos/issues/33), [ADR-022](../03-adr/ADR-022-libctos-crt.md)). Not app hosting. |
| A3 | ELF (or raw) loader into user TTBR0 | **Loader mile Verified** ([#34](https://github.com/artofdream/ctos/issues/34), [ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)). Not a Linux ABI. Not app hosting. |
| A4 | Standing EL0 as normal mode | **Standing-task mile Verified** on this tip ([#35](https://github.com/artofdream/ctos/issues/35), [ADR-024](../03-adr/ADR-024-standing-el0-normal.md)). Loaded app until `exit`; fail-closed restore. Not isolation. Not app hosting. |
| A5 | Isolation completion (remaining identity tear; PAN only if CPU + ADR) | **Identity `.rodata` tear Verified** on this tip ([#36](https://github.com/artofdream/ctos/issues/36), [ADR-025](../03-adr/ADR-025-identity-rodata-tear.md)). PAN **enable** stays **Planned** with CPU evidence ([ADR-026](../03-adr/ADR-026-pan-capability.md): `pan: absent` on `-cpu cortex-a57`). Not “EL0 isolated.” A6–A9 stay **Planned**. |
| A6 | Thin VFS + memfs | **Planned** ([#37](https://github.com/artofdream/ctos/issues/37)) |
| A7 | virtio-blk + FAT or xv6-like | **Planned** ([#38](https://github.com/artofdream/ctos/issues/38)) |
| A8 | Documented sample apps | **Planned** ([#39](https://github.com/artofdream/ctos/issues/39)) |
| A9 | Disconnect OS image from app payloads | **Planned** after A1–A4 ([#48](https://github.com/artofdream/ctos/issues/48)). Today: one linked ELF — not Verified. |

## Out of scope for Track A

OCI/Docker containers, glibc/musl ports, SMP, networking (unless a later ADR). See [host-apps.md](../framework/host-apps.md).

Samples today: site [what-can-run.md](../overview/what-can-run.md) (extra: [apps-today.md](../framework/apps-today.md)). Porting: site [porting.md](../overview/porting.md) (extra: [building-or-porting.md](../framework/building-or-porting.md)).
