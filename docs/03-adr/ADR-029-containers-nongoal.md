# ADR-029 — Guest OCI/Docker containers are a non-goal

- Status: Accepted (docs decision; no guest runtime)
- Date: 2026-09-12

## Context

Track B ([issue #40](https://github.com/artofdream/ctos/issues/40)) is Linux-compat **research**. Child [B7 / #47](https://github.com/artofdream/ctos/issues/47) asks for an explicit claim: ctos does **not** host Docker/OCI; today Docker on a Linux host runs the ctos *smoke* image.

Site pages already said **no** ([hosting-apps.md](../overview/hosting-apps.md), extra [host-apps.md](../framework/host-apps.md)). Track B still listed B7 as **Planned** and said containers “stay out or far-later.” That is an honesty gap: **far-later** reads like a later Planned feature.

OCI / Docker / Kubernetes guests need a Linux host: namespaces, cgroups, a Linux ABI, usually overlay or equivalent, and a rich syscall surface. This learning kernel does not have those. Host `docker-smoke.sh` is a **harness** (NFR-04 / NFR-05), not a guest runtime.

This ADR does **not** implement a container runtime. It does **not** mint FR-16+ or NFR-15+. B1 (Linux-compat goals) may cite it and must not reopen containers as Planned.

## Decision

1. **Non-goal.** The ctos guest is not a container host. OCI / Docker / Kubernetes workloads on the guest are **not Planned**. Do not write a container roadmap on Track B.
2. **Arrow today.** Docker (or Podman) on a Linux host may **build and QEMU-smoke** this kernel. That is “Docker hosts ctos,” not “ctos hosts containers.” Ledger Docker rows stay host-harness probes.
3. **Reopen gate.** Far-later only with a **new GitHub epic** plus a new ADR that names the missing Linux-host features and a probe. Track B B1–B6 must not treat containers as a win or a silent Planned row. Do not mint a new FR/NFR ID for this.
4. **Principles drive.** Honesty (NFR-06): absence of OCI/runc/cgroup/namespace code in `src/` is the probe. Antifragility keeps the host Docker ratchet. Security / performance claims about “container isolation” stay unwritten.

## Consequences

- Docs: [track-b.md](../04-roadmap/track-b.md) B7 becomes **Documented**. Site SoT: [hosting-apps.md](../overview/hosting-apps.md). Extra: [host-apps.md](../framework/host-apps.md). Ledger row stays **Verified** absence + non-goal.
- Track A A8 (sample apps) is unrelated. Do not add an in-tree “container” sample.
- “Run Alpine on ctos” / “k8s node” stays the same class of claim as “POSIX port is easy.” It is not.
