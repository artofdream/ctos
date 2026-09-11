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
| A1 | Stable SVC / syscall ABI + docs | **Planned** ([#32](https://github.com/artofdream/ctos/issues/32)) |
| A2 | Freestanding CRT / `libctos` | **Planned** ([#33](https://github.com/artofdream/ctos/issues/33)) |
| A3 | ELF (or raw) loader into user TTBR0 | **Planned** ([#34](https://github.com/artofdream/ctos/issues/34)) |
| A4 | Standing EL0 as normal mode | **Planned** ([#35](https://github.com/artofdream/ctos/issues/35)) |
| A5 | Isolation completion (remaining identity tear; PAN only if CPU + ADR) | **Planned** ([#36](https://github.com/artofdream/ctos/issues/36)) |
| A6 | Thin VFS + memfs | **Planned** ([#37](https://github.com/artofdream/ctos/issues/37)) |
| A7 | virtio-blk + FAT or xv6-like | **Planned** ([#38](https://github.com/artofdream/ctos/issues/38)) |
| A8 | Documented sample apps | **Planned** ([#39](https://github.com/artofdream/ctos/issues/39)) |
| A9 | Disconnect OS image from app payloads | **Planned** after A1–A4 ([#48](https://github.com/artofdream/ctos/issues/48)). Today: one linked ELF — not Verified. |

## Out of scope for Track A

OCI/Docker containers, glibc/musl ports, SMP, networking (unless a later ADR). See [host-apps.md](../framework/host-apps.md).

Samples today: site [what-can-run.md](../overview/what-can-run.md) (extra: [apps-today.md](../framework/apps-today.md)). Porting: site [porting.md](../overview/porting.md) (extra: [building-or-porting.md](../framework/building-or-porting.md)).
