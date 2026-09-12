# ADR-031 — Linux-compat goals, non-goals, keep ctos specificity

- Status: Accepted (docs frame; no Linux userspace claim)
- Date: 2026-09-12

## Context

Track B ([issue #40](https://github.com/artofdream/ctos/issues/40)) is Linux-compat **research**. Child [B1 / #41](https://github.com/artofdream/ctos/issues/41) asks for the frame: what “Linux-compat” means here (ABI **subset** vs a full Linux kernel), what stays out, and which ctos invariants must not move.

[B7 / ADR-029](ADR-029-containers-nongoal.md) already records that guest OCI / Docker / Kubernetes is a **non-goal**. This ADR cites that decision. It does **not** reopen containers as Planned or “far-later on this track.”

Track A ([issue #31](https://github.com/artofdream/ctos/issues/31)) has first cuts A1–A9: a tiny SVC ABI ([ADR-021](ADR-021-svc-syscall-abi.md)), `libctos` ([ADR-022](ADR-022-libctos-crt.md)), a freestanding ELF64 `PT_LOAD` loader that **rejects `PT_INTERP`** ([ADR-023](ADR-023-elf-pt-load-loader.md)), standing EL0 ([ADR-024](ADR-024-standing-el0-normal.md)), thin VFS + memfs ([ADR-027](ADR-027-thin-vfs-memfs.md)), virtio-blk + FAT16 ([ADR-028](ADR-028-virtio-blk-fat16.md)), sample recipes, and an OS/app slot first cut ([ADR-030](ADR-030-os-app-slots.md)). Those miles are **freestanding**. They are not a Linux ABI and not Linux userspace.

Issue #40’s reality check stands: a full Linux ABI (and containers) is a multi-year kernel project. Track B is an ADR / research ladder, not a promise to become Linux.

This ADR does **not** implement B2–B6. It does **not** mint FR-16+ or NFR-15+. File presence of this page is not a Linux userspace probe.

## Hypotheses

| ID | Option | Notes |
| --- | --- | --- |
| **H1 (chosen)** | Frame Linux-compat as **research** into a possible ABI **subset**. Keep the freestanding Track A path. Containers stay [ADR-029](ADR-029-containers-nongoal.md). Say “not claiming Linux userspace yet.” | Matches #40 / #41. Leaves B2–B6 room to map gaps and then decide. |
| **H2 (rejected)** | Promise a full Linux ABI, glibc/musl, or “runs Alpine / busybox.” | Honesty fail (NFR-06). No probe exists. |
| **H3 (rejected)** | Reopen guest containers as Planned or “far-later” on Track B. | Conflicts with ADR-029. Reopen only with a **new** epic + ADR. |
| **H4 (rejected)** | Replace Track A SVC / `libctos` with Linux AArch64 `svc #0` / `x8` numbers in this mile. | Would break probed A1–A9 markers for a research frame. B2 maps the gap; it does not rewrite `src/`. |
| **H5 (rejected)** | Treat A3 `PT_LOAD` + A9 FAT `/hello` as Linux userspace. | The loader rejects `PT_INTERP`. Standing EL0 is not `execve`. File presence is not that claim. |

## Decision

1. **What Linux-compat means.** On ctos it means: **research** whether a **subset** of the Linux AArch64 ABI could ever be useful for a *named, probed* workload, **without** dropping ctos specificity. It is **not** a promise to run a Linux distro, glibc, musl, a shell, Python, or “Linux userspace.” Full Linux ABI is out of scope for this track’s *promise*. B6 may later choose “never.”

2. **Goals (research ladder only).** Later children stay document-first. This PR does not write them.

   | Later | Job | Must not do in that child |
   | --- | --- | --- |
   | B2 ([#42](https://github.com/artofdream/ctos/issues/42)) | Map Linux aarch64 syscalls vs today’s ctos SVC (16–23 + reserved 0–2) | Do not add Linux syscall numbers to `src/` |
   | B3 ([#43](https://github.com/artofdream/ctos/issues/43), [ADR-035](ADR-035-process-model-standing-el0.md)) | Process model vs Linux `fork` / `exec` / `wait` | Do not add those syscalls. Standing EL0 stays the ctos model. |
   | B4 ([#44](https://github.com/artofdream/ctos/issues/44)) | Linux ELF / auxv / `PT_INTERP` vs the freestanding loader | Do not accept `PT_INTERP`. Gap ADR: [ADR-033](ADR-033-linux-elf-auxv-pt-interp.md). |
   | B5 ([#45](https://github.com/artofdream/ctos/issues/45)) | Linux VFS concepts vs the thin ctos VFS | Do not grow POSIX `open` flags / dentries |
   | B6 ([#46](https://github.com/artofdream/ctos/issues/46)) | Decision: compat layer vs reimplement vs **never** | Do not treat “compat layer” as already chosen |

   Implementation, if any, comes **after** B6 names a path, in a later epic, with a probe. “Never” is a valid B6 outcome.

3. **Non-goals.**

   - Full Linux ABI / POSIX product / glibc or musl ports.
   - Guest OCI / Docker / Kubernetes — already [ADR-029](ADR-029-containers-nongoal.md). **Not Planned** on Track B.
   - “Run Alpine on ctos,” “k8s node,” “Linux userspace works.”
   - Replacing the freestanding Track A path (A1–A9 first cuts stay).
   - Widening off QEMU `virt` / AArch64 without a new ISA ADR ([ADR-003](ADR-003-primary-isa-aarch64.md)).
   - New FR/NFR IDs. Frozen set stays [fr-nfr.md](../02-requirements/fr-nfr.md).
   - Author self-merge ([ADR-002](ADR-002-pr-identity-split.md)).

4. **Keep (ctos specificity).** Track B is **subordinate** to [principles.md](../framework/principles.md). Linux-compat research must not override:

   | Invariant | Why it stays |
   | --- | --- |
   | Honesty (NFR-06 / [ADR-001](ADR-001-honesty-harness-for-ctos.md)) | Status words need a probe. Unprobed stays **Unknown**. This ADR is not Verified Linux. |
   | Antifragility (NFR-05) | Fail-closed `qemu-smoke` and host Docker/cts-ai ratchets stay. Host `docker-smoke.sh` is still “Docker hosts ctos,” not a guest Linux ABI. |
   | Security (NFR-10) | No “secure OS,” “Linux-compat isolation,” or “the kernel is W^X” from a research frame. |
   | Performance (NFR-07) | No invented benches for a compat layer that does not exist. Measure first if a later mile adds a path. |
   | Document-first / one-PR loops | One child → one branch → one GitHub PR. |
   | Freestanding Track A path | In-tree apps keep SVC `#n`, `libctos`, `PT_LOAD` without interpreter, thin VFS. A research map must not silently retarget those numbers to Linux `x8`. |
   | Virt learning scope | QEMU `virt` + PL011 until an ADR widens it. |
   | ADR-gated memory / EL0 | Isolation, PAN enable, remaining identity RAM / `_start` stay their own rows. |

5. **Relationship to ADR-029.** Guest containers remain a **non-goal**. B1–B6 must not treat containers as a win, a silent Planned row, or a reason to grow namespaces / cgroups / overlay. Reopen only with a **new GitHub epic** plus a new ADR that names the missing Linux-host features and a probe. Do not mint an FR/NFR ID for that.

6. **Not claiming Linux userspace yet.** Say that sentence in the roadmap and on the Track B page. Today:

   - A3 maps freestanding ELF64 `ET_EXEC` `PT_LOAD` and **rejects `PT_INTERP`**.
   - Standing EL0 is not a Linux process table.
   - Thin VFS + memfs / read-only FAT16 is not Linux VFS.
   - A9 FAT `/hello` is not `execve`.
   - The ledger row “Guest runs host apps (Linux ELF / shell / Python)” stays **Planned**. This ADR does **not** change that row to Verified.

7. **Principles drive.** Do not override honesty, fail-closed sensors, or the three pillars for a “runs Linux” headline.

## Consequences

- Docs: [track-b.md](../04-roadmap/track-b.md) B1 becomes **Documented**. [SUMMARY.md](../SUMMARY.md) and [roadmap.md](../04-roadmap/roadmap.md) link here. Site extra: [hosting-apps.md](../overview/hosting-apps.md) / [host-apps.md](../framework/host-apps.md) point at the frame without a userspace claim.
- B2 is **Documented** (syscall gap note). B3 is **Documented** ([ADR-035](ADR-035-process-model-standing-el0.md)). B4 is **Documented** ([ADR-033](ADR-033-linux-elf-auxv-pt-interp.md)). B5 / B6 stay **Planned** research. No `src/` change in this PR.
- Track A remains the product path for in-tree freestanding apps. Cross-update and “app hosting is done” stay unclaimed.
- Honesty: document inspection only. Do not add a Verified Linux-userspace ledger row because this file exists.
