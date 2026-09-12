# Gaps to host apps — and no containers

Do not say “you can run host apps on ctos.” A **host app** here means a program you already run on Linux or macOS: a shell, Python, a browser, a Docker/OCI container, a glibc ELF you `exec`. Today’s guest can run the [probed samples](apps-today.md) only. Closing the gap is **Planned** in pieces, not a product claim.

`./scripts/docker-smoke.sh` is a **host** harness (build the kernel inside a Linux container, then QEMU). It is not a guest container runtime. ctos is **not** a container host.

Hub: [overview.md](overview.md). Porting: [building-or-porting.md](building-or-porting.md). Filesystem: [filesystem.md](filesystem.md).

Site source of truth: [Hosting apps / containers](../overview/hosting-apps.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## What a host app needs vs what exists

| Need (typical host app) | On ctos today | Status |
| --- | --- | --- |
| A process you `exec` | One linked kernel ELF. Standing EL0 is a dual-SVC stub plus A1 kernel ABI, an A2 `libctos` hello, an A3 guest `PT_LOAD` of that same embedded ELF, and an A4 standing **task** until `exit` | **Planned** (slots / A9). A1–A8 are ABI + CRT + loader + standing-task + VFS + documented recipes. Not a Linux `exec`. |
| POSIX / glibc / musl | Custom `aarch64-ctos.json`, `os: none`, no libc | Not easy, not started. |
| Files (`open` / a disk) | Thin VFS: memfs + read-only FAT16 | **memfs + FAT miles** (A6/A7). Not POSIX. |
| Sockets / HTTP | No virtio-net, no stack | **Planned** at best; not a Now mile |
| Shell / TTY / Python | One injected UART byte; no interpreter | **No** until ABI + FS + line discipline |
| Isolated userspace | First miles + live `.text` tear; PAN / full teardown missing | Isolation **Planned** |
| Preemption / SMP | Cooperative EL1, one vCPU | **No** on this horizon |
| Containers (OCI / Docker / k8s **as the guest**) | Nothing | **Non-goal.** See below. |

The easiest thing you can add today is still an in-tree `no_std` coop EL1 task — not a host binary.

## Containers: non-goal

ctos will **not** run containers as a guest feature. This is a **non-goal**, not “later on this horizon.” [ADR-029](../03-adr/ADR-029-containers-nongoal.md). Site SoT: [hosting-apps.md](../overview/hosting-apps.md).

- No OCI image pull, no `runc`, no cgroups, no Linux namespaces, no overlay FS, no containerd/CRI.
- A Linux Docker/Podman host that *builds* this kernel is unrelated. That smoke does not make QEMU `virt` a container host.
- “Run Alpine on ctos” / “k8s node” is the same class of claim as “POSIX port is easy.” It is not.

Do not write a container roadmap. Track A gaps (ABI, FS, isolation) are not a path to OCI. Do not mint a new FR ID for containers. Reopen only with a new GitHub epic plus a new ADR.

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| Guest is not a container host | Source: no OCI/runc/cgroup/namespace code in `src/` | Verified (absence) |
| Host `docker-smoke.sh` builds the kernel | Ledger Docker rows (sponsor / GHA) | Separate claim — host harness only |
| A host app (Linux ELF, shell, Python) runs in the guest | No such serial marker | **Planned** |
| Guest is a container host | No OCI/runc/cgroup code | **Verified** absence; **non-goal** ([ADR-029](../03-adr/ADR-029-containers-nongoal.md); not a later Planned feature) |

File presence of this note is not an app runtime. See the [honesty ledger](honesty-ledger.md).
