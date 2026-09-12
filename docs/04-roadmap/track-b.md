# Track B — Linux-compat research (keep ctos specificity)

Tracker epic: [issue #40](https://github.com/artofdream/ctos/issues/40). Child issues B1–B7.

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Linux-compat research must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “runs Linux” headline.

- Honesty ledger — Verified only with a probe
- Antifragility — fail-closed smoke + ratchets
- Security — threat model; probed mitigations only
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

Track A ([track-a.md](track-a.md)) should largely complete before deep Linux ABI work. Reuse A1–A7 where they help. Full Linux ABI is not a promise. **Guest OCI/Docker/k8s is a non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)) — not a later Planned row on this track.

## Goal

Explore what Linux userspace / ABI-subset work would take **without** dropping ctos specificity. This is an ADR/research ladder, not a distro.

## Keep

Honesty ledger, fail-closed smoke (including Docker/cts-ai ratchets), pillars (antifragility / security / performance), ADR-gated ISA/memory/EL0 decisions, QEMU `virt` until an ADR widens it.

## Ordered research

| ID | Work | Status (2026-09-12) |
| --- | --- | --- |
| B1 | ADR: Linux-compat goals & non-goals | **Planned** ([#41](https://github.com/artofdream/ctos/issues/41)). Must cite [ADR-029](../03-adr/ADR-029-containers-nongoal.md); do not reopen containers as Planned. |
| B2 | Syscall surface map (Linux aarch64 vs ctos SVC) | **Planned** ([#42](https://github.com/artofdream/ctos/issues/42)) |
| B3 | Process model vs Linux (fork/exec/wait) | **Planned** ([#43](https://github.com/artofdream/ctos/issues/43)) |
| B4 | Linux ELF / auxv / `PT_INTERP` vs freestanding loader | **Planned** ([#44](https://github.com/artofdream/ctos/issues/44)) |
| B5 | Linux VFS concepts vs thin ctos VFS | **Planned** ([#45](https://github.com/artofdream/ctos/issues/45)) |
| B6 | Decision: compat layer vs reimplement vs never | **Planned** ([#46](https://github.com/artofdream/ctos/issues/46)) |
| B7 | Containers remain a non-goal (OCI needs a Linux host) | **Documented** ([#47](https://github.com/artofdream/ctos/issues/47), [ADR-029](../03-adr/ADR-029-containers-nongoal.md)). Site: [hosting-apps.md](../overview/hosting-apps.md) (extra: [host-apps.md](../framework/host-apps.md)). Ledger: absence **Verified**; not Planned. |

## B7 — containers are a non-goal

**ctos guests do not host containers.** OCI / Docker / Kubernetes need a Linux kernel (namespaces, cgroups, a Linux ABI, usually overlay). This tree has none of that in `src/`.

**Today the arrow is reversed:** a Linux host may run Docker/`docker-smoke.sh` to **build and QEMU-smoke** the ctos image. That is a host harness (NFR-04 / NFR-05), not a guest runtime.

Do not claim a Docker/OCI host. Do not treat this as far-later work waiting on Track B. Reopen only with a **new GitHub epic** plus a new ADR — not a Track B “win,” and not a new FR/NFR ID.
