# Track B — Linux-compat research (keep ctos specificity)

Tracker epic: [issue #40](https://github.com/artofdream/ctos/issues/40). Child issues B1–B7.

## Driving force (non-negotiable)

This track is **subordinate** to ctos core principles ([principles.md](../framework/principles.md)). Linux-compat research must **not** abandon honesty, fail-closed sensors, the three pillars, or virt learning scope. Do not override principles for a “runs Linux” headline.

- Honesty ledger — Verified only with a probe
- Antifragility — fail-closed smoke + ratchets
- Security — threat model; probed mitigations only
- Performance — measure first; no invented benches
- Document-first / one milestone → one branch → one PR

Track A ([track-a.md](track-a.md)) should largely complete before deep Linux ABI work. Reuse A1–A9 where they help. Full Linux ABI is not a promise. **Not claiming Linux userspace yet** ([ADR-031](../03-adr/ADR-031-linux-compat-goals.md)). **Guest OCI/Docker/k8s is a non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)) — not a later Planned row on this track.

## Goal

Explore what Linux userspace / ABI-subset work would take **without** dropping ctos specificity. Frame: [ADR-031](../03-adr/ADR-031-linux-compat-goals.md). This is an ADR/research ladder, not a distro.

## Keep

Honesty ledger, fail-closed smoke (including Docker/cts-ai ratchets), pillars (antifragility / security / performance), ADR-gated ISA/memory/EL0 decisions, QEMU `virt` until an ADR widens it.

## Ordered research

| ID | Work | Status (2026-09-12) |
| --- | --- | --- |
| B1 | ADR: Linux-compat goals & non-goals | **Documented** ([#41](https://github.com/artofdream/ctos/issues/41), [ADR-031](../03-adr/ADR-031-linux-compat-goals.md)). ABI **subset** research; keep ctos specificity; **not claiming Linux userspace yet**. Cites [ADR-029](../03-adr/ADR-029-containers-nongoal.md); does not reopen containers as Planned. |
| B2 | Syscall surface map (Linux aarch64 vs ctos SVC) | **Documented** ([#42](https://github.com/artofdream/ctos/issues/42), [linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md)). Inspection map only. No Linux numbers in `src/`. **Not claiming Linux userspace.** |
| B3 | Process model vs Linux (fork/exec/wait) | **Planned** ([#43](https://github.com/artofdream/ctos/issues/43)) |
| B4 | Linux ELF / auxv / `PT_INTERP` vs freestanding loader | **Documented** ([#44](https://github.com/artofdream/ctos/issues/44), [ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md)). Gap ADR only. Track A loader stays reusable. **Does not accept `PT_INTERP`.** **Not claiming dynamic Linux ELF.** |
| B5 | Linux VFS concepts vs thin ctos VFS | **Planned** ([#45](https://github.com/artofdream/ctos/issues/45)) |
| B6 | Decision: compat layer vs reimplement vs never | **Planned** ([#46](https://github.com/artofdream/ctos/issues/46)) |
| B7 | Containers remain a non-goal (OCI needs a Linux host) | **Documented** ([#47](https://github.com/artofdream/ctos/issues/47), [ADR-029](../03-adr/ADR-029-containers-nongoal.md)). Site: [hosting-apps.md](../overview/hosting-apps.md) (extra: [host-apps.md](../framework/host-apps.md)). Ledger: absence **Verified**; not Planned. |

## B1 — Linux-compat frame

[ADR-031](../03-adr/ADR-031-linux-compat-goals.md) is the Track B frame. Linux-compat here means **research** into a possible Linux AArch64 ABI **subset**, not a promise to run a distro, glibc, musl, a shell, or Linux userspace.

**Keep:** honesty ledger, fail-closed smoke (including host Docker/cts-ai ratchets), three pillars, the freestanding Track A path (A1–A9 first cuts), QEMU `virt` until an ADR widens it.

**Out:** full Linux ABI as a promise; guest containers ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)); replacing SVC `#n` / `libctos` with Linux `x8` numbers in a research mile; claiming userspace because A3 loads ELF64 or A9 reads FAT `/hello`.

B2 is **Documented** (syscall gap table). B4 is **Documented** (ELF / auxv / `PT_INTERP` gap). B3 / B5 / B6 stay **Planned** research. They compare process / VFS concepts and then decide (compat layer vs reimplement vs **never**). This page does not implement them. File presence of ADR-031, the B2 note, or ADR-033 is not a Linux userspace probe.

## B2 — syscall gap map

[linux-aarch64-syscall-gap.md](../research/linux-aarch64-syscall-gap.md) maps a representative Linux AArch64 set (`exit`, `write`, `read`, `openat`, `close`, `brk`/`mmap`, `clone`/`execve`/`wait4`, `ioctl`, plus CRT/namespace neighbors) onto today’s ctos SVC surface (reserved 0–2, public 16–23).

**Keep:** no Linux `svc #0` / `x8` numbers in `src/` this mile. Track A `SVC #<n>` stays. **No row is `present`** (convention + number space both miss). Related Track A SVCs are **partial**. Namespace/mount stay **never-per-ADR-031** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md)).

B3–B6 must not treat this table as an implementation backlog. B6 may choose **never**.

## B4 — Linux ELF / auxv / PT_INTERP vs the freestanding loader

[ADR-033](../03-adr/ADR-033-linux-elf-auxv-pt-interp.md) compares Linux `exec` of an ELF (interpreter + auxv + `PT_DYNAMIC`) with today’s A3 loader ([ADR-023](../03-adr/ADR-023-elf-pt-load-loader.md)).

**Keep:** the Track A parser reusable (`ET_EXEC`, `PT_LOAD`, reject `PT_INTERP`, reject W+X). In-tree `libctos` apps stay on that path.

**Out:** accepting `PT_INTERP` in `src/`; claiming musl/glibc or dynamic Linux ELF; treating A9 FAT `/hello` as `execve`.

Static musl still needs a Linux stack/auxv and the B2 syscall surface. Dynamic musl/glibc also need an interpreter. **Not claiming dynamic Linux ELF.** B3 / B5 / B6 stay Planned. B6 may choose **never**.

## B7 — containers are a non-goal

**ctos guests do not host containers.** OCI / Docker / Kubernetes need a Linux kernel (namespaces, cgroups, a Linux ABI, usually overlay). This tree has none of that in `src/`.

**Today the arrow is reversed:** a Linux host may run Docker/`docker-smoke.sh` to **build and QEMU-smoke** the ctos image. That is a host harness (NFR-04 / NFR-05), not a guest runtime.

Do not claim a Docker/OCI host. Do not treat this as far-later work waiting on Track B. Reopen only with a **new GitHub epic** plus a new ADR — not a Track B “win,” and not a new FR/NFR ID.
