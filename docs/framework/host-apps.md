# Gaps to host apps — and no containers

Do not say “you can run host apps on ctos.” A **host app** here means a program you already run on Linux or macOS: a shell, Python, a browser, a Docker/OCI container, a glibc ELF you `exec`. Today’s guest can run the [probed samples](apps-today.md) only. Closing the gap is **Planned** in pieces, not a product claim.

`./scripts/docker-smoke.sh` is a **host** harness (build the kernel inside a Linux container, then QEMU). It is not a guest container runtime. ctos is **not** a container host.

Hub: [overview.md](overview.md). Porting: [building-or-porting.md](building-or-porting.md). Filesystem: [filesystem.md](filesystem.md).

Site source of truth: [Hosting apps / containers](../overview/hosting-apps.md). This page is extra stance. HTTPS at https://ctos.artof.link is **Verified** (2026-09-11 after #30).

## What a host app needs vs what exists

| Need (typical host app) | On ctos today | Status |
| --- | --- | --- |
| A process you `exec` | One linked kernel ELF. Standing EL0 is a dual-SVC stub plus A1 kernel ABI (`exit` / `uart_write` / `yield`) | **Planned** (loader + CRT). A1 is the ABI mile only. |
| POSIX / glibc / musl | Custom `aarch64-ctos.json`, `os: none`, no libc | Not easy, not started. |
| Files (`open` / a disk) | No VFS | **Planned** memfs → virtio-blk → FAT/xv6-like |
| Sockets / HTTP | No virtio-net, no stack | **Planned** at best; not a Now mile |
| Shell / TTY / Python | One injected UART byte; no interpreter | **No** until ABI + FS + line discipline |
| Isolated userspace | First miles + live `.text` tear; PAN / full teardown missing | Isolation **Planned** |
| Preemption / SMP | Cooperative EL1, one vCPU | **No** on this horizon |
| Containers (OCI / Docker / k8s **as the guest**) | Nothing | **No.** See below. |

The easiest thing you can add today is still an in-tree `no_std` coop EL1 task — not a host binary.

## Containers: no

ctos will **not** run containers as a guest feature on this horizon.

- No OCI image pull, no `runc`, no cgroups, no Linux namespaces, no overlay FS, no containerd/CRI.
- A Linux Docker/Podman host that *builds* this kernel is unrelated. That smoke does not make QEMU `virt` a container host.
- “Run Alpine on ctos” / “k8s node” is the same class of claim as “POSIX port is easy.” It is not.

Do not write a container roadmap that skips process ABI, a filesystem, and isolation. Those are earlier **Planned** gaps. Do not mint a new FR ID for containers.

## Honesty

| Claim | Probe | Status |
| --- | --- | --- |
| Guest is not a container host | Source: no OCI/runc/cgroup/namespace code in `src/` | Verified (absence) |
| Host `docker-smoke.sh` builds the kernel | Ledger Docker rows (sponsor / GHA) | Separate claim — host harness only |
| A host app (Linux ELF, shell, Python) runs in the guest | No such serial marker | **Planned** |
| Guest is a container host | No OCI/runc/cgroup code | **Verified** absence; **non-goal** (not a later Planned feature) |

File presence of this note is not an app runtime. See the [honesty ledger](honesty-ledger.md).
