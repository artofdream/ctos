# Track B — Linux-compat research (keep ctos specificity)

Tracker epic: [issue #40](https://github.com/artofdream/ctos/issues/40). Child issues B1–B7.

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Linux-compat research must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “runs Linux” headline.

- Honesty ledger — Verified only with a probe
- Antifragility — fail-closed smoke + ratchets
- Security — threat model; probed mitigations only
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

Track A ([track-a.md](track-a.md)) should largely complete before deep Linux ABI work. Reuse A1–A7 where they help. Full Linux ABI + containers is not a promise.

## Goal

Explore what Linux userspace / ABI-subset work would take **without** dropping ctos specificity. This is an ADR/research ladder, not a distro.

## Keep

Honesty ledger, fail-closed smoke (including Docker/cts-ai ratchets), pillars (antifragility / security / performance), ADR-gated ISA/memory/EL0 decisions, QEMU `virt` until an ADR widens it.

## Ordered research

| ID | Work | Status (2026-09-11) |
| --- | --- | --- |
| B1 | ADR: Linux-compat goals & non-goals | **Planned** ([#41](https://github.com/artofdream/ctos/issues/41)) |
| B2 | Syscall surface map (Linux aarch64 vs ctos SVC) | **Planned** ([#42](https://github.com/artofdream/ctos/issues/42)) |
| B3 | Process model vs Linux (fork/exec/wait) | **Planned** ([#43](https://github.com/artofdream/ctos/issues/43)) |
| B4 | Linux ELF / auxv / `PT_INTERP` vs freestanding loader | **Planned** ([#44](https://github.com/artofdream/ctos/issues/44)) |
| B5 | Linux VFS concepts vs thin ctos VFS | **Planned** ([#45](https://github.com/artofdream/ctos/issues/45)) |
| B6 | Decision: compat layer vs reimplement vs never | **Planned** ([#46](https://github.com/artofdream/ctos/issues/46)) |
| B7 | Containers remain a non-goal (OCI needs a Linux host) | **Planned** as documentation ([#47](https://github.com/artofdream/ctos/issues/47)); guest runtime already **no** in site [hosting-apps.md](../overview/hosting-apps.md) (extra: [host-apps.md](../framework/host-apps.md)) |

Do not claim Docker/OCI host without namespaces/cgroups/overlay. That stays out or far-later — not a Track B “win.”
